use std::{
    fs::{create_dir, exists, File},
    path::PathBuf,
};

use ant_host_agent::client::AntHostAgentClientConfig;
use ant_library::host_architecture::HostArchitecture;
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use tar::Archive;
use tempfile::tempdir_in;
use tokio::fs::create_dir_all;
use tracing::info;

use crate::{
    err::AntZookeeperError,
    routes::service::{artifact_file_name, artifact_persist_dir},
    state::AntZookeeperState,
};

pub(super) fn envs_persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("envs")
}

pub(super) fn envs_file_name(project: &str, environment: &str) -> String {
    format!("{project}.{environment}.build.cfg")
}

pub(super) fn services_persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("services-db")
}

pub(super) fn services_file_name(
    project: &str,
    arch: Option<&HostArchitecture>,
    version: &str,
) -> String {
    format!("{}.deployable", artifact_file_name(project, arch, version))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentRequest {
    pub project: String,
    pub version: String,
    pub host: String,
}

async fn create_deployment(
    State(state): State<AntZookeeperState>,
    Json(req): Json<CreateDeploymentRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.get_project(&req.project).await? {
        return Err(AntZookeeperError::validation_msg(
            format!("No such project: {}", req.project).as_str(),
        ));
    }

    let (_, host_arch) = match state.db.get_host(&req.host).await? {
        None => {
            return Err(AntZookeeperError::validation_msg(
                format!("No such host: {}", req.host).as_str(),
            ));
        }
        Some(host) => host,
    };

    let (_, environment) = match state
        .db
        .find_host_group_by_host_and_project(&req.host, &req.project)
        .await?
    {
        None => {
            return Err(AntZookeeperError::validation_msg(
                format!(
                    "Host {} is not a part of the deployment path of project {}",
                    req.host, req.project
                )
                .as_str(),
            ));
        }
        Some(h) => h,
    };

    let env_file_path =
        envs_persist_dir(&state.root_dir).join(envs_file_name(&req.project, &environment));

    let (_, artifact_relative_path) = match state
        .db
        .get_artifact(&req.project, Some(&host_arch), &req.version)
        .await?
    {
        None => {
            return Err(AntZookeeperError::validation_msg(
                format!(
                    "No artifact found for {} version {} on architecture {}",
                    req.project,
                    req.version,
                    host_arch.as_str()
                )
                .as_str(),
            ));
        }
        Some(artifact) => artifact,
    };

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
            archive.unpack(&unpack_dir_path)?;

            // Inject the right environment variables with a .env into project
            if exists(&env_file_path)? {
                std::fs::copy(env_file_path, unpack_dir_path.join(".env"))?;
            }

            unpack_dir_path
        };

        // Create a new tarball with the new files injected
        let pack_file_path = {
            let pack_file_path = dir.path().join("pack.tar");

            let pack_file = File::create_new(&pack_file_path)?;
            let mut archive = tar::Builder::new(GzEncoder::new(pack_file, Compression::default()));

            archive.append_dir_all(".", &unpack_dir_path)?;
            archive.finish()?;

            pack_file_path
        };

        create_dir_all(services_persist_dir(&state.root_dir)).await?;
        let service_file_path = services_persist_dir(&state.root_dir).join(services_file_name(
            &req.project,
            Some(&host_arch),
            &req.version,
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
    let ant_host_agent = state
        .ant_host_agent_factory
        .lock()
        .await
        .new_client(AntHostAgentClientConfig {
            endpoint: req.host,
            port: 3232,
        })
        .await
        .unwrap();

    ant_host_agent
        .register_service(&req.project, &req.version, service_file)
        .await?;

    ant_host_agent
        .install_service(ant_host_agent::routes::service::InstallServiceRequest {
            project: req.project.clone(),
            version: req.version.clone(),
            is_docker: Some(false),
            secrets: Some(vec![]),
        })
        .await?;

    Ok(())
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new().route_with_tsr("/deployment", post(create_deployment))
}
