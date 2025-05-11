use crate::types::{DbRouter, DbState};
use ant_data_farm::{hosts::HostId, DaoTrait};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum_extra::routing::RouterExt;
use uuid::Uuid;

async fn host(
    Path(host_id): Path<Uuid>,
    State(dao): DbState,
) -> Result<impl IntoResponse, HostsError> {
    let hosts = dao.hosts.read().await;
    let host = hosts.get_one_by_id(&HostId(host_id)).await?;

    match host {
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(format!("Host with ID '{host_id}' not found!")).into_response(),
        )),
        Some(host) => Ok((StatusCode::OK, Json(host).into_response())),
    }
}

async fn list_all(State(dao): DbState) -> Result<impl IntoResponse, HostsError> {
    let hosts = dao.hosts.read().await;
    let all_hosts = hosts.get_all().await?;
    Ok((StatusCode::OK, Json(all_hosts).into_response()))
}

pub fn router() -> DbRouter {
    Router::new()
        .route_with_tsr("/host/{host_id}", get(host))
        .route_with_tsr("/list-all", get(list_all))
        // .route_with_tsr("/register-host", post(register_host))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "GET /host/{host_id}",
                "GET /list-all",
                "POST /register-host",
            ])
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
