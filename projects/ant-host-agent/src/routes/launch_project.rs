use std::str::FromStr;

use ant_metadata::Project;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use axum_typed_multipart::TypedMultipart;
use hyper::StatusCode;
use tracing::info;

use crate::{
    common::launch_project::LaunchProjectRequest,
    procs::launch_project::{launch_project, LaunchProjectError},
};

impl IntoResponse for LaunchProjectError {
    fn into_response(self) -> Response {
        match self {
            LaunchProjectError::LaunchBinary(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(format!("Launch failed: {}", e.to_string()).to_string()),
                )
                    .into_response()
            }
            LaunchProjectError::SaveArtifact(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(e.to_string()).into_response(),
                )
                    .into_response()
            }
            LaunchProjectError::AlreadyExists => {
                return (
                    StatusCode::OK,
                    Json("Project already launched!".to_string()),
                )
                    .into_response()
            }
        };
    }
}

pub async fn launch_project_route(
    TypedMultipart(req): TypedMultipart<LaunchProjectRequest>,
) -> impl IntoResponse {
    info!("Artifact :::");
    info!(
        "file_name = '{}'",
        req.artifact.metadata.file_name.unwrap_or(String::new())
    );
    info!(
        "content_type = '{}",
        req.artifact
            .metadata
            .content_type
            .unwrap_or(String::from("text/plain")),
    );
    info!("len = '{}", req.artifact.contents.len());

    let project = match Project::from_str(req.project.as_str()) {
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json("Project invalid!").into_response(),
            )
        }
        Ok(p) => p,
    };

    let res = launch_project(project, req.artifact.contents).await;
    return match res {
        Ok(r) => (StatusCode::OK, Json(r).into_response()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.into_response()),
    };
}
