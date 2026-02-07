use std::{
    fs::{exists, File},
    io::Read,
    path::PathBuf,
};

use ant_host_agent::client::AntHostAgentClientConfig;
use ant_zoo_storage::HostGroup;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use tar::Archive;
use tempfile::tempdir_in;
use tokio::fs::{create_dir_all, OpenOptions};
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
    let manifest: AnthillManifest = get_manifest_from_file(&dest.join("anthill.json"))?;

    create_dir_all(dest.join("secrets")).await?;

    for secret in manifest.secrets.unwrap_or_default() {
        info!("Copying secret {secret}");
        tokio::fs::copy(
            secret_file_path(&state.root_dir, environment, &secret),
            dest.join("secrets").join(secret_file_name(&secret)),
        )
        .await?;
    }

    Ok(())
}

async fn inject_env_file(
    state: &AntZookeeperState,
    dest: &PathBuf,
    project: &str,
    environment: &str,
) -> Result<(), anyhow::Error> {
    let source_env_file_paths = vec![
        envs_persist_dir(&state.root_dir).join(global_envs_file_name(&environment)),
        envs_persist_dir(&state.root_dir).join(project_envs_file_name(&project, &environment)),
    ];

    let mut env_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(dest.join(".env"))
        .await?;
    for path in source_env_file_paths {
        if exists(&path)? {
            info!(
                "Copying env config from [{}] to [{}]",
                path.display(),
                dest.join(".env").display()
            );
            let mut source = tokio::fs::File::open(path).await?;
            tokio::io::copy(&mut source, &mut env_file).await?;
        } else {
            info!("No such env config [{}]", path.display());
        }
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
        let dir = tempdir_in(&state.root_dir.join("tmp"))?;

        // Unpack to a directory
        let unpack_dir_path = {
            info!("Unpacking tarball to [{}].", artifact_path.display());
            let artifact = File::open(&artifact_path)?;
            let gz = GzDecoder::new(&artifact);
            let mut archive = Archive::new(gz);

            let unpack_dir_path = dir.path().join("unpack");
            archive.unpack(&unpack_dir_path)?;

            inject_env_file(state, &unpack_dir_path, project, &host_group.environment).await?;
            inject_secrets(state, &unpack_dir_path, &host_group.environment).await?;

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
            is_docker: Some(false),
            secrets: Some(vec![]),
        })
        .await?;

    Ok(())
}
