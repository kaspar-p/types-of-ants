use std::{collections::HashSet, os::unix::fs::PermissionsExt, str::FromStr};

use ant_library::{
    host_architecture::HostArchitecture, manifest_file::ManifestFile, services::Services,
};
use anthill_manifest::{AnthillArchetype, AnthillBuild, AnthillBuildParallelism, AnthillManifest};
use anyhow::Context;
use async_tempfile::TempFile;
use bollard::body_full;
use clap_complete::engine::ArgValueCompleter;
use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

use crate::{complete::complete_projects, git::GitState};

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
}

impl BuildCmd {
    pub fn new(project: String, arch: Option<HostArchitecture>) -> Self {
        Self { project, arch }
    }
}

pub struct DeploymentFile {
    pub file: TempFile,
    pub version: String,
    pub arch: HostArchitecture,
}

pub async fn build(cmd: BuildCmd) -> Vec<DeploymentFile> {
    let services: Services =
        serde_json::de::from_reader(std::fs::File::open(find_up("services.json")).unwrap())
            .unwrap();
    services.validate().expect("malformed services.json");

    let arches: HashSet<HostArchitecture> = if let Some(arch) = cmd.arch.as_ref() {
        [arch.clone()].into()
    } else {
        services
            .hosts
            .values()
            .filter(|h| !h.ineligible_for_deployments())
            .filter(|h| h.services.iter().any(|s| s.project == cmd.project))
            .map(|h| h.architecture.clone())
            .collect()
    };

    if arches.is_empty() {
        panic!("No architectures found for project, is it in services.json?");
    }

    let git = GitState::new().expect("failed to fetch git state");
    let project_src = git.root.join("projects").join(&cmd.project);
    let manifest = AnthillManifest::from_file(&project_src.join("anthill.json")).expect(&format!(
        "Project {} had no anthill.json in its root.",
        cmd.project
    ));

    match manifest.build_parallelism {
        AnthillBuildParallelism::Serial => {
            let mut files = vec![];
            for arch in arches {
                let cmd2 = cmd.clone();
                let git2 = git.clone();
                let proj = cmd2.project.clone();
                files.push(
                    build_arch(cmd2, &arch, &git2)
                        .await
                        .context(format!("building {} {}", proj, arch.as_str()))
                        .expect("build failed"),
                )
            }

            return files;
        }

        AnthillBuildParallelism::Parallel => {
            let mut handles = Vec::new();
            for arch in arches {
                let git2 = git.clone();
                let cmd2 = cmd.clone();
                let proj = cmd2.project.clone();
                let handle = tokio::task::spawn_blocking(|| async move {
                    build_arch(cmd2, &arch, &git2).await.context(format!(
                        "building {} {}",
                        proj,
                        arch.as_str()
                    ))
                });
                handles.push(handle);
            }

            let handles = futures::future::join_all(handles)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .expect("task scheduling failed")
                .into_iter();

            let files = futures::future::join_all(handles)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .expect("build failed");

            files
        }
    }
}

#[tracing::instrument(skip(cmd, git))]
async fn build_arch<'a>(
    cmd: BuildCmd,
    arch: &'a HostArchitecture,
    git: &'a GitState,
) -> Result<DeploymentFile, anyhow::Error> {
    info!("BUILDING [{}] for [{}]...", cmd.project, arch.as_str());

    let deployment_file = build_artifact(&cmd, &arch, &git)
        .await
        .context("build failed")?;

    Ok(DeploymentFile {
        version: git.version(),
        file: deployment_file,
        arch: arch.clone(),
    })
}

async fn build_artifact<'a>(
    cmd: &'a BuildCmd,
    arch: &'a HostArchitecture,
    git: &'a GitState,
) -> Result<TempFile, anyhow::Error> {
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

    if !std::fs::exists(project_src.join("Makefile"))? {
        return Err(anyhow::anyhow!("No Makefile in {}", project_src.display()));
    }

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
            tracing::error!("ANT-ERR-086: make failed");
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

    let anthill = AnthillManifest::from_file(&project_src.join("anthill.json"))
        .context("read anthill.json")?;

    // docker-image.tar: Copy image if the project is a Docker one.
    if matches!(anthill.build, AnthillBuild::Docker) {
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

        let image_name = format!("{}:{}", cmd.project, git.version());

        // Build Docker image
        {
            let build_context_tar = tempfile::NamedTempFile::new()?;
            let mut tar_builder = tar::Builder::new(&build_context_tar);
            {
                let walker = ignore::WalkBuilder::new(&project_src).build();

                for result in walker {
                    let entry = result?;
                    let path = entry.path();

                    // Skip explicitly ignored paths or errors
                    if !path.exists() {
                        warn!("ANT-ERR-087: Ignoring {}", path.display());
                        continue;
                    }

                    if path.is_file() {
                        info!("Adding {}", path.display());
                        let relative_path = pathdiff::diff_paths(path, &project_src).unwrap();
                        tar_builder.append_path_with_name(path, relative_path)?;
                    } else if path.is_dir() {
                    } else {
                        warn!("ANT-ERR-088: Ignoring {}", path.display());
                    }
                }
            }
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
                    "Docker image tarball is {}, exceeding the limit.",
                    humansize::format_size(contents.len(), humansize::DECIMAL)
                )));
            } else {
                info!(
                    "Docker image tarball is {} bytes",
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
                        error!("ANT-ERR-089: Error while building docker image: {}", e);
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

    // manifest.json: Deprecated, old commit version.
    {
        let mut manifest =
            tokio::fs::File::create_new(tmp_packaging_dir.path().join("manifest.json"))
                .await
                .context("creating manifest.json")?;
        let content = ManifestFile {
            commit_number: git.head_number.to_string(),
        };
        let content = serde_json::to_string(&content)?;
        manifest
            .write_all(content.as_bytes())
            .await
            .context("writing manifest.json")?;
    }

    // VERSION: create client-side version file
    {
        let mut manifest = tokio::fs::File::create_new(tmp_packaging_dir.path().join("VERSION"))
            .await
            .context("creating VERSION")?;
        manifest
            .write_all(git.version().as_bytes())
            .await
            .context("writing VERSION")?;
    }

    // run.sh: Copy runtime file
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

    // .db-migrations: Copy database migrations if they are a db
    {
        match anthill.archetype {
            Some(AnthillArchetype::Postgres { migration_dir, .. }) => {
                let migrations_path = project_src.join(migration_dir);
                if !std::fs::exists(&migrations_path)? {
                    return Err(anyhow::anyhow!(
                        "Migrations directory not found: {}",
                        migrations_path.display()
                    ));
                }
            }
            _ => {}
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

    // anthill.json: Copy anthill manifest
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

        info!("... building deployment file");

        let deployment_file = async_tempfile::TempFile::builder()
            .dir(&registry_dir)
            .prefix("depl")
            .create()
            .await?;

        {
            let path = deployment_file.file_path().to_owned();
            let file = std::fs::File::create(&path)?;

            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut tar = tar::Builder::new(encoder);

            tar.append_dir_all(".", tmp_packaging_dir.path())?;
            tar.finish()?;

            let encoder = tar.into_inner()?;
            encoder.finish()?;
        }

        info!(
            "... deployment file size: {}",
            humansize::format_size(
                std::fs::metadata(&deployment_file.file_path())
                    .context("reading deployment file")?
                    .len(),
                humansize::DECIMAL
            )
        );

        deployment_file
    };

    Ok(deployment_file)
}
