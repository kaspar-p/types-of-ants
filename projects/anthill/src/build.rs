use std::{collections::HashSet, os::unix::fs::PermissionsExt, path::PathBuf, str::FromStr};

use ant_library::{host_architecture::HostArchitecture, services::Services};
use ant_zookeeper::{client::AntZookeeperClientConfig, routes::service::UpsertRevisionRequest};
use anthill_manifest::{AnthillBuild, AnthillBuildParallelism, AnthillManifest};
use anyhow::Context;
use bollard::body_full;
use chrono::{Datelike, Timelike};
use clap::ArgAction;
use clap_complete::engine::ArgValueCompleter;
use futures::StreamExt;
use git2::{Commit, Repository};
use serde_json::json;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info};

use crate::complete::complete_projects;

pub fn find_up(filename: &str) -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap();

    loop {
        let candidate = dir.join(filename);
        if std::fs::exists(&candidate).unwrap() {
            return candidate;
        }

        dir = dir
            .parent()
            .expect(&format!("got to root without finding: {filename}"))
            .to_path_buf()
    }
}

#[derive(Clone, clap::Args)]
pub struct BuildCmd {
    #[arg(add = ArgValueCompleter::new(complete_projects))]
    project: String,

    #[arg(short, long, value_parser = HostArchitecture::from_str)]
    arch: Option<HostArchitecture>,

    /// By default, deploy after building
    #[clap(long = "no-deploy", action = ArgAction::SetFalse)]
    deploy: bool,

    /// Or choose --no-deploy to not deploy.
    #[clap(long = "deploy", overrides_with = "deploy")]
    _no_deploy: bool,
}

pub async fn build(cmd: BuildCmd) {
    let services: Services =
        serde_json::de::from_reader(std::fs::File::open(find_up("services.json")).unwrap())
            .unwrap();
    services.validate().expect("malformed services.json");

    let client = ant_zookeeper::client::AntZookeeperClient::new(AntZookeeperClientConfig {
        tls: false,
        endpoint: "localhost:3235".to_string(),
    });

    let revision = if cmd.deploy {
        Some(
            client
                .upsert_revision(UpsertRevisionRequest {
                    project: cmd.project.clone(),
                })
                .await
                .unwrap(),
        )
    } else {
        None
    };

    let arches: HashSet<HostArchitecture> = cmd
        .clone()
        .arch
        .map(|a| HashSet::from([a]))
        .unwrap_or_else(|| {
            services
                .hosts
                .iter()
                .map(|(id, _)| id)
                .map(|id| services.hosts.get(id).unwrap().architecture.clone())
                .collect()
        });

    let git = GitState::new().unwrap();
    let project_src = git.root.join("projects").join(&cmd.project);
    let manifest = AnthillManifest::from_file(&project_src.join("anthill.json")).expect(&format!(
        "Project {} had no anthill.json in its root.",
        cmd.project
    ));

    match manifest.build_parallelism {
        AnthillBuildParallelism::Serial => {
            for arch in arches {
                let revision: Option<String> = revision.as_ref().map(|r| r.revision.clone());
                let cmd2 = cmd.clone();
                let proj = cmd2.project.clone();
                build_arch(cmd2, &arch, revision.as_deref())
                    .await
                    .context(format!("building {} {}", proj, arch.as_str()))
                    .expect("build failed");
            }
        }

        AnthillBuildParallelism::Parallel => {
            let mut handles = Vec::new();
            for arch in arches {
                let revision: Option<String> = revision.as_ref().map(|r| r.revision.clone());
                let cmd2 = cmd.clone();
                let proj = cmd2.project.clone();
                let handle = tokio::task::spawn_blocking(|| async move {
                    build_arch(cmd2, &arch, revision.as_deref())
                        .await
                        .context(format!("building {} {}", proj, arch.as_str()))
                });
                handles.push(handle);
            }

            let handles = futures::future::join_all(handles)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .expect("task scheduling failed")
                .into_iter();

            futures::future::join_all(handles)
                .await
                .into_iter()
                .collect::<Result<Vec<()>, _>>()
                .expect("build failed");
        }
    }
}

fn commit_count(repo: &Repository) -> Result<i32, anyhow::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    Ok(revwalk.count() as i32)
}

fn get_head_commit_data<'a>(repo: &'a Repository) -> Result<Commit<'a>, anyhow::Error> {
    let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
    let commit = obj
        .into_commit()
        .map_err(|_| git2::Error::from_str("Not a commit"))?;

    Ok(commit)
}

fn format_datetime(t: git2::Time) -> String {
    let d = chrono::DateTime::from_timestamp_secs(t.seconds()).expect("malformed datetime");
    format!(
        "{}-{}-{}-{}-{}",
        d.year(),
        d.month(),
        d.day(),
        d.hour(),
        d.minute()
    )
}

async fn build_arch<'a>(
    cmd: BuildCmd,
    arch: &'a HostArchitecture,
    revision: Option<&'a str>,
) -> Result<(), anyhow::Error> {
    info!("BUILDING [{}] for [{}]...", cmd.project, arch.as_str());

    let (deployment_file, version) = build_artifact(&cmd, &arch).await.context("build failed")?;

    info!("... registering artifact");

    if let Some(revision) = revision {
        let client = ant_zookeeper::client::AntZookeeperClient::new(AntZookeeperClientConfig {
            tls: false,
            endpoint: "localhost:3235".to_string(),
        });
        client
            .register_artifact(
                &revision,
                &cmd.project,
                &arch,
                &version,
                &deployment_file.path(),
            )
            .await
            .context("register artifact")?;

        info!("... artifact registered.");
    } else {
        let (_, path) = deployment_file.keep()?;
        info!("artifact available: {}", path.display());
    }

    Ok(())
}

pub struct GitState {
    pub root: PathBuf,
    head_sha: String,
    head_number: i32,
    head_datetime: String,
}

impl GitState {
    pub fn new() -> Result<Self, anyhow::Error> {
        let repo = git2::Repository::discover(".")?;
        let commit = get_head_commit_data(&repo)?;

        Ok(Self {
            root: repo.workdir().unwrap().to_path_buf(),
            head_sha: commit.id().to_string().chars().take(8).collect(),
            head_number: commit_count(&repo)?,
            head_datetime: format_datetime(commit.time()),
        })
    }
}

async fn build_artifact<'a>(
    cmd: &'a BuildCmd,
    arch: &'a HostArchitecture,
) -> Result<(NamedTempFile, String), anyhow::Error> {
    let git = GitState::new()?;

    let version = format!("{}-{}-{}", git.head_number, git.head_datetime, git.head_sha);

    let project_src = git.root.join("projects").join(&cmd.project);
    if !std::fs::exists(&project_src).expect("project src exists") {
        return Err(anyhow::Error::msg(format!(
            "project directory {} does not exist",
            project_src.display()
        )));
    }

    let build_dir = project_src.join("build");
    let build_output_dir = build_dir.join("release").join(arch.as_str());
    match std::fs::remove_dir_all(&build_output_dir) {
        Ok(_) => Ok(()),
        Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => Ok(()),
        Err(e) => Err(e),
    }
    .context(format!(
        "removing build dir: {}",
        build_output_dir.display()
    ))?;
    tokio::fs::create_dir_all(&build_output_dir)
        .await
        .context("creating build dir")?;

    {
        let exit = tokio::process::Command::new("make")
            .args(["-C", project_src.to_str().unwrap()])
            .args([
                "-e",
                &format!(
                    "BUILD_OUTPUT_DIR={}",
                    build_output_dir.as_os_str().to_str().unwrap()
                ),
            ])
            .args(["-e", &format!("ARCH={}", arch.as_str())])
            .args(["-e", &format!("RUST_TARGET={}", arch.rust_target())])
            .args(["-e", &format!("PROMETHEUS_OS={}", arch.prometheus_os())])
            .args(["-e", &format!("PROMETHEUS_ARCH={}", arch.prometheus_arch())])
            .args(["-e", &format!("HASHICORP_ARCH={}", arch.hashicorp_arch())])
            .args(["-e", &format!("ALLOY_ARCH={}", arch.alloy_arch())])
            .args(["-e", &format!("commit_sha={}", git.head_sha)])
            .arg("release")
            .spawn()
            .context("starting make cmd failed")?
            .wait()
            .await
            .context("make cmd failed")?;

        if !exit.success() {
            tracing::error!("make failed");
            return Err(anyhow::Error::msg("make failed, see logs"));
        }
    }

    // Copy all build results in
    let tmp_packaging_dir = {
        let tmp_build_dir = build_dir.join(format!("tmp.project.build"));
        tokio::fs::create_dir_all(&tmp_build_dir)
            .await
            .context("creating tmp dir")?;

        let tmp_packaging_dir =
            tempfile::tempdir_in(&tmp_build_dir).context("creating packaging dir")?;
        dircpy::copy_dir(&build_output_dir, &tmp_packaging_dir.path())
            .context(format!(
                "copying build output from {} to {}",
                build_output_dir.display(),
                tmp_packaging_dir.path().display()
            ))
            .context("copying tmp packaging")?;
        tmp_packaging_dir
    };

    // Handle Docker projects
    if matches!(
        AnthillManifest::from_file(&project_src.join("anthill.json"))
            .context("read anthill.json")?
            .build,
        AnthillBuild::Docker
    ) {
        info!("... creating docker image");

        let compose_file_path = git
            .root
            .join("projects")
            .join("ant-zookeeper")
            .join("dev-fs")
            .join("dev-fs")
            .join("envs")
            .join("docker-compose.yml");

        tokio::fs::copy(
            &compose_file_path,
            &tmp_packaging_dir.path().join("docker-compose.yml"),
        )
        .await
        .context("copy docker-compose.yml")?;

        let docker = bollard::Docker::connect_with_defaults().context("connect to daemon")?;

        let image_name = format!("{}:{}", cmd.project, version);

        // Build Docker image
        {
            let build_context_tar = tempfile::NamedTempFile::new()?;
            let mut tar_builder = tar::Builder::new(&build_context_tar);
            tar_builder
                .append_dir_all(".", &project_src)
                .context("add dockerfile to build context tar")?;
            tar_builder.finish().context("write build context tar")?;

            let mut contents = Vec::new();
            tokio::fs::File::open(&build_context_tar)
                .await
                .context("open docker tar")?
                .read_to_end(&mut contents)
                .await
                .context("read docker tar")?;

            const SIZE_MAX: usize = 1000 * 1000 * 1000; // 1gb
            if contents.len() > SIZE_MAX {
                return Err(anyhow::Error::msg(format!(
                    "Docker image is {}, exceeding the limit.",
                    humansize::format_size(contents.len(), humansize::DECIMAL)
                )));
            } else {
                info!(
                    "Docker image is {} bytes",
                    humansize::format_size(contents.len(), humansize::DECIMAL)
                );
            }

            let mut stream = docker.build_image(
                bollard::query_parameters::BuildImageOptionsBuilder::new()
                    .t(&image_name)
                    .build(),
                None,
                Some(body_full(contents.into())),
            );

            while let Some(build_log) = stream.next().await {
                match build_log {
                    Ok(output) => {
                        if let Some(stream) = output.stream {
                            info!("{}", stream);
                        }
                    }

                    Err(e) => {
                        error!("Error while building docker image: {}", e);
                        return Err(Into::<anyhow::Error>::into(e)).context("docker build failed");
                    }
                }
            }
        }

        // Save docker image
        {
            info!("... exporting docker image");
            let image_path = tmp_packaging_dir.path().join("docker-image.tar");

            let mut stream = docker.export_image(&image_name);
            let mut file = tokio::fs::File::create_new(image_path)
                .await
                .context("create docker image")?;

            while let Some(chunk) = stream.next().await {
                let bytes = chunk?;
                file.write_all(&bytes)
                    .await
                    .context("write docker image chunk")?;
            }
        }
    }

    // Create manifest
    {
        let mut manifest =
            tokio::fs::File::create_new(tmp_packaging_dir.path().join("manifest.json"))
                .await
                .context("creating manifest")?;
        let content = serde_json::to_string(&json!({ "commit_number": git.head_number }))?;
        manifest
            .write_all(content.as_bytes())
            .await
            .context("writing manifest")?;
    }

    // Copy run.sh
    {
        let run_path = project_src.join(".anthill").join("run.sh");
        if std::fs::exists(&run_path)? {
            tokio::fs::copy(run_path, tmp_packaging_dir.path().join("run.sh"))
                .await
                .context("copying run.sh")?;

            let mut perms =
                std::fs::metadata(tmp_packaging_dir.path().join("run.sh"))?.permissions();
            perms.set_mode(perms.mode() | 0o111);
            std::fs::set_permissions(tmp_packaging_dir.path().join("run.sh"), perms)
                .context("setting run.sh executable")?;
        } else {
            return Err(anyhow::Error::msg("Project needs run.sh"));
        }
    }

    // // Copy systemd
    // {
    //     let systemd_path = project_src.join(format!("{}.service", cmd.project));
    //     if std::fs::exists(&systemd_path)? {
    //         tokio::fs::copy(
    //             systemd_path,
    //             tmp_packaging_dir
    //                 .path()
    //                 .join(format!("{}.service", cmd.project)),
    //         )
    //         .await
    //         .context("copying systemd")?;
    //     }
    // }

    // Copy anthill manifest
    {
        tokio::fs::copy(
            project_src.join("anthill.json"),
            tmp_packaging_dir.path().join("anthill.json"),
        )
        .await
        .context("copying anthill manifest")?;
    }

    // Create deployment file
    let deployment_file = {
        let registry_dir = git.root.join("build").join("registry");
        tokio::fs::create_dir_all(&registry_dir)
            .await
            .context("creating registry")?;

        let deployment_file_name = format!("{}.{}.{}.tar.gz", cmd.project, arch.as_str(), version);

        info!("... building deployment file: {deployment_file_name}");

        let deployment_file_path = registry_dir.join(deployment_file_name);
        let deployment_file = NamedTempFile::new_in(&registry_dir)
            .context("creating deployment file")?;

        {
            let gz = flate2::write::GzEncoder::new(
                deployment_file.as_file().try_clone().context("cloning deployment file")?,
                flate2::Compression::default(),
            );
            let mut tar = tar::Builder::new(gz);
            tar.append_dir_all(".", tmp_packaging_dir.path())
                .context("appending packaging to tar")?;
            tar.finish().context("building tar")?;
        }

        info!(
            "... deployment file size: {}",
            humansize::format_size(
                std::fs::metadata(&deployment_file_path)
                    .context("reading deployment file")?
                    .len(),
                humansize::DECIMAL
            )
        );

        deployment_file
    };

    Ok((deployment_file, version))
}
