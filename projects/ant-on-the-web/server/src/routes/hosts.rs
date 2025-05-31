use crate::types::{ApiRouter, ApiState, InnerApiState};
use ant_data_farm::{
    hosts::{Host, HostId},
    DaoTrait,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum_extra::routing::RouterExt;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetHostResponse {
    pub host: Host,
}

async fn host(
    Path(host_identifier): Path<String>,
    State(InnerApiState { dao, sms: _ }): ApiState,
) -> Result<impl IntoResponse, HostsError> {
    let hosts = dao.hosts.read().await;

    info!("Trying parse as UUID and ID: {}", host_identifier);
    if let Ok(host_id) = Uuid::parse_str(host_identifier.as_str()) {
        if let Some(host) = hosts.get_one_by_id(&HostId(host_id)).await? {
            return Ok((StatusCode::OK, Json(GetHostResponse { host })).into_response());
        }
    }

    info!("Trying as hostname: {}", host_identifier);
    if let Some(host) = hosts.get_one_by_hostname(host_identifier.as_str()).await? {
        return Ok((StatusCode::OK, Json(GetHostResponse { host })).into_response());
    }

    return Ok((
        StatusCode::NOT_FOUND,
        format!("Host with ID or hostname '{host_identifier}' not found!"),
    )
        .into_response());
}

#[derive(Serialize, Deserialize)]
pub struct GetHostsResponse {
    pub hosts: Vec<Host>,
}

async fn all_hosts(
    State(InnerApiState { dao, sms: _ }): ApiState,
) -> Result<impl IntoResponse, HostsError> {
    let hosts = dao.hosts.read().await;
    let all_hosts = hosts.get_all().await?;
    Ok((
        StatusCode::OK,
        Json(GetHostsResponse { hosts: all_hosts }).into_response(),
    ))
}

pub fn router() -> ApiRouter {
    Router::new()
        .route_with_tsr("/host/{host}", get(host))
        .route_with_tsr("/hosts", get(all_hosts))
        .fallback(|| async {
            ant_library::api_fallback(&["GET /host/{host_id_or_name}", "GET /hosts"])
        })
}

struct HostsError(anyhow::Error);

impl IntoResponse for HostsError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for HostsError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
