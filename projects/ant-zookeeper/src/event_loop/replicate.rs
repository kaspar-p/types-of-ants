use std::{
    collections::HashMap,
    fs::{exists, File},
    path::PathBuf,
};

use ant_host_agent::client::AntHostAgentClientConfig;
use ant_zookeeper_db::HostGroup;
use anyhow::Context;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use tar::Archive;
use tempfile::tempdir_in;
use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::AsyncWriteExt,
};
use tracing::info;

use crate::{
    anthill::{get_manifest_from_file, AnthillManifest},
    fs::{
        artifact_persist_dir, envs_persist_dir, global_envs_file_name, project_envs_file_name,
        secret_file_name, secret_file_path, services_file_name, services_persist_dir,
    },
    state::AntZookeeperState,
};

async fn inject_secrets(
    state: &AntZookeeperState,
    dest: &PathBuf,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("Injecting secrets...");
    let manifest: AnthillManifest = get_manifest_from_file(&dest.join("anthill.json"))?;

    let project_secrets_dir = dest.join("secrets");
    create_dir_all(&project_secrets_dir).await?;

    for secret in manifest.secrets.unwrap_or_default() {
        let src_file = secret_file_path(&state.root_dir, environment, &secret);
        let dest_file = project_secrets_dir.join(secret_file_name(&secret));
        info!(
            "Copying secret: [{}] -> [{}]",
            src_file.display(),
            dest_file.display()
        );
        tokio::fs::copy(src_file, dest_file).await?;
    }

    Ok(())
}

fn escape_value(val: &str) -> String {
    let val = val.replace("\"", "\\\"");
    format!("\"{}\"", val)
}

fn source_env_variables(
    state: &AntZookeeperState,
    project: &str,
    version: &str,
    environment: &str,
) -> Result<Vec<(String, String)>, anyhow::Error> {
    let source_files = vec![
        envs_persist_dir(&state.root_dir).join(global_envs_file_name(&environment)),
        envs_persist_dir(&state.root_dir).join(project_envs_file_name(&project, &environment)),
    ];

    let mut variables = HashMap::<String, String>::new();
    for path in source_files {
        let entries = match dotenvy::from_path_iter(&path) {
            Err(dotenvy::Error::Io(io_err))
                if matches!(io_err.kind(), std::io::ErrorKind::NotFound) =>
            {
                Ok(vec![])
            }

            Err(e) => Err(e).context(format!("reading env: {}", path.display())),
            Ok(f) => Ok(f
                .into_iter()
                .filter_map(|e| match e {
                    Err(_) => None,
                    Ok(t) => Some(t),
                })
                .collect::<Vec<(String, String)>>()),
        }?;

        for (k, v) in entries {
            variables.insert(k, escape_value(&v));
        }
    }

    variables.insert(
        "PERSIST_DIR".to_string(),
        escape_value(&format!("/home/ant/persist/{project}")),
    );
    variables.insert("SECRETS_DIR".to_string(), escape_value("./secrets"));
    variables.insert("VERSION".to_string(), escape_value(version));

    let mut variables = variables.into_iter().collect::<Vec<(String, String)>>();

    variables.sort();

    Ok(variables)
}

/// One of the contracts is that docker-compose files that look like:
///     volume: "{{PERSIST_DIR}}/my-dir:/app/dir"
/// will eventually be replaced by the right PERSIST_DIR variable according to the environment:
///     volume: "/persist/my-dir:/app/dir"
/// So we need to mustache-replace the docker-compose.yml file that we find
async fn render_docker_compose(
    state: &AntZookeeperState,
    dest: &PathBuf,
    project: &str,
    version: &str,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("Rendering docker-compose file...");

    let docker_compose_path = dest.join("docker-compose.yml");

    if !exists(&docker_compose_path)? {
        info!("No docker compose file found.");
        return Ok(());
    }

    let template = mustache::compile_path(&docker_compose_path)?;

    // Create in a temp place in case something fails
    let variables = mustache::to_data(source_env_variables(state, project, version, environment)?)?;

    info!("Interpolating variables...");
    let mut docker_compose_file = File::create(dest.join(".__docker-compose.yml"))?;
    template.render_data(&mut docker_compose_file, &variables)?;

    info!("Renaming temp file .__docker-compose.yml to docker-compose.yml");
    std::fs::rename(
        dest.join(".__docker-compose.yml"),
        dest.join("docker-compose.yml"),
    )?;

    Ok(())
}

async fn inject_env_file(
    state: &AntZookeeperState,
    dest: &PathBuf,
    project: &str,
    version: &str,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("Creating .env file...");

    let mut env_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(dest.join(".env"))
        .await?;
    for (key, value) in source_env_variables(state, project, version, environment)? {
        let content = format!("{}={}\n", key, value);
        info!("Writing trimmed config [{}]", content.trim());

        env_file.write_all(content.as_bytes()).await?;
    }

    Ok(())
}

pub async fn replicate_artifact_step(
    state: &AntZookeeperState,
    project: &str,
    revision: &str,
    host_group: &HostGroup,
    host: &str,
) -> Result<(), anyhow::Error> {
    let (_, host_arch) = state.db.get_host(&host).await?.expect("host exists");

    let (_, version, artifact_relative_path) = state
        .db
        .get_artifact_by_revision(Some(&host_arch), &revision)
        .await?
        .unwrap();

    let artifact_path = artifact_persist_dir(&state.root_dir).join(artifact_relative_path);

    // Construct a new tarball using the build artifact in artifact_path and inject .env and other files.
    let service_file_path = {
        let mut dir = tempdir_in(&state.root_dir.join("tmp"))?;
        dir.disable_cleanup(true);

        // Unpack to a directory
        let unpack_dir_path = {
            let artifact = File::open(&artifact_path)?;
            let gz = GzDecoder::new(&artifact);
            let mut archive = Archive::new(gz);

            let unpack_dir_path = dir.path().join("unpack");
            info!(
                "Unpacking tarball [{}] to [{}]",
                artifact_path.display(),
                unpack_dir_path.display()
            );
            archive.unpack(&unpack_dir_path)?;

            inject_env_file(
                state,
                &unpack_dir_path,
                project,
                &version,
                &host_group.environment,
            )
            .await?;
            inject_secrets(state, &unpack_dir_path, &host_group.environment).await?;
            render_docker_compose(
                state,
                &unpack_dir_path,
                project,
                &version,
                &host_group.environment,
            )
            .await?;

            unpack_dir_path
        };

        // Create a new tarball with the new files injected
        let pack_file_path = {
            let pack_file_path = dir.path().join("pack.tar");
            info!("Repacking tarball to [{}].", pack_file_path.display());

            let pack_file = File::create_new(&pack_file_path)?;
            let mut archive = tar::Builder::new(GzEncoder::new(pack_file, Compression::default()));

            archive.append_dir_all(".", &unpack_dir_path)?;
            archive.finish()?;

            pack_file_path
        };

        create_dir_all(services_persist_dir(&state.root_dir)).await?;
        let service_file_path = services_persist_dir(&state.root_dir).join(services_file_name(
            &project,
            Some(&host_arch),
            &version,
        ));
        info!(
            "Copying tarball to final location [{}]",
            service_file_path.display()
        );
        std::fs::copy(pack_file_path, &service_file_path)?;

        service_file_path
    };

    let service_file = File::open(service_file_path)?;

    // Send the service file to ant-host-agent
    let ant_host_agent =
        state
            .ant_host_agent_factory
            .lock()
            .await
            .new_client(AntHostAgentClientConfig {
                endpoint: host.to_string(),
                port: 3232,
            });

    info!("Replicating service file to: {host}");
    ant_host_agent
        .register_service(&project, &version, service_file)
        .await?;

    info!("Installing service file file to: {host}");
    ant_host_agent
        .install_service(ant_host_agent::routes::service::InstallServiceRequest {
            project: project.to_string(),
            version: version.to_string(),
        })
        .await?;

    Ok(())
}
