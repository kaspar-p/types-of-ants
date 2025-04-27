use crate::artifact::artifact::{Artifact, ArtifactBuildError};
use ant_host_agent::{
    clients::{Host, HostAgentClient},
    kill_project::KillStatus,
    launch_project::LaunchStatus,
};
use ant_metadata::{Architecture, ArtifactSelection, Project};
use axum::{
    extract::Json,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Deserialize)]
pub struct DeployProjectRequest {
    pub project: Project,
    pub selection: ArtifactSelection,
    pub architecture: Architecture,
}

#[derive(Debug)]
pub enum DeployProjectError {
    Build(ArtifactBuildError),
    Connect,
    Kill(KillStatus),
    Launch(LaunchStatus),
}

impl IntoResponse for DeployProjectError {
    fn into_response(self) -> Response {
        match self {
            DeployProjectError::Build(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Building artifact failed!"),
            )
                .into_response(),
            DeployProjectError::Connect => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to connect to dataplane host"),
            )
                .into_response(),
            DeployProjectError::Kill(_) | DeployProjectError::Launch(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to launch new process on dataplane host"),
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeployProjectResponse {
    pub deployed: chrono::DateTime<chrono::offset::Utc>,
    pub host: Host,
}

#[debug_handler]
pub async fn deploy_project(
    Json(artifact_request): Json<DeployProjectRequest>,
) -> Result<Json<DeployProjectResponse>, DeployProjectError> {
    let artifact = match Artifact::new(
        artifact_request.project,
        artifact_request.architecture,
        artifact_request.selection,
    )
    .get()
    .await
    {
        Err(e) => {
            debug!("Error in building artifact: {e}");
            return Err(DeployProjectError::Build(e));
        }
        Ok(artifact) => artifact,
    };

    let host = Host::new(
        "localhost".to_owned(),
        dotenv::var("HOST_AGENT_PORT")
            .expect("No HOST_AGENT_PORT environment variable found")
            .parse::<u16>()
            .expect("HOST_AGENT_PORT was not u16"),
    );
    let daemon = match HostAgentClient::connect(host.clone()) {
        Err(e) => {
            debug!("Failed to connect to host agent daemon! Is it running? Error: {e}");
            return Err(DeployProjectError::Connect);
        }
        Ok(daemon) => daemon,
    };

    let kill_response = match daemon.kill_project(artifact_request.project).await {
        Err(e) => {
            debug!("Error killing project: {e}");
            return Err(DeployProjectError::Connect);
        }
        Ok(res) => res,
    };

    match kill_response.status {
        KillStatus::NothingToKill => debug!("There was nothing to kill, starting launch!"),
        KillStatus::Successful => debug!("Killed process, starting launch!"),
        KillStatus::Unsuccessful => {
            debug!("Failed to kill process! See logs for more!");
            return Err(DeployProjectError::Kill(KillStatus::Unsuccessful));
        }
    };

    let launch_response = match daemon
        .launch_project(artifact_request.project, &artifact.path)
        .await
    {
        Err(e) => {
            debug!("Failed to launch project: {e}");
            return Err(DeployProjectError::Connect);
        }
        Ok(launch) => launch,
    };

    match launch_response.status {
        LaunchStatus::LaunchSuccessful => {
            debug!("Launched!")
        }
    }

    return Ok(Json(DeployProjectResponse {
        host: host.clone(),
        deployed: chrono::offset::Utc::now(),
    }));
}
