use axum::{response::IntoResponse, Json};
use hyper::StatusCode;

use crate::{
    common::kill_project::KillProjectRequest,
    procs::kill_project::{kill_project, KillProjectError},
};

pub async fn kill_project_route(
    Json(kill_project_req): Json<KillProjectRequest>,
) -> impl IntoResponse {
    let project_str = kill_project_req.project.as_str().clone();
    let res = kill_project(kill_project_req).await;

    return match res {
        Err(e) => match e {
            KillProjectError::NothingToKill => (
                StatusCode::NOT_FOUND,
                Json(format!("No project {} found to kill!", project_str)).into_response(),
            ),
            KillProjectError::FailedToKill => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Failed to kill project {}!", project_str)).into_response(),
            ),
        },

        Ok(r) => (StatusCode::OK, Json(r).into_response()),
    };
}
