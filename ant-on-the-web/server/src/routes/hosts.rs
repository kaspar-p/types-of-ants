use crate::{
    middleware::fallback::fallback,
    types::{DaoRouter, DaoState},
};
use ant_data_farm::{hosts::HostId, DaoTrait};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use uuid::Uuid;

async fn host(Path(host_id): Path<Uuid>, State(dao): DaoState) -> impl IntoResponse {
    let hosts = dao.hosts.read().await;
    let host = hosts.get_one_by_id(&HostId(host_id)).await;

    match host {
        None => (
            StatusCode::NOT_FOUND,
            Json(format!("Host with ID '{}' not found!", host_id)).into_response(),
        ),
        Some(host) => (StatusCode::OK, Json(host).into_response()),
    }
}

async fn register_host() -> impl IntoResponse {
    (StatusCode::OK, Json("New host registered!"))
}

async fn list_all(State(dao): DaoState) -> impl IntoResponse {
    let hosts = dao.hosts.read().await;
    (StatusCode::OK, Json(hosts.get_all().await).into_response())
}

pub fn router() -> DaoRouter {
    Router::new()
        .route_with_tsr("/host/:host-id", get(host))
        .route_with_tsr("/list-all", get(list_all))
        .route_with_tsr("/register-host", post(register_host))
        .fallback(|| async {
            fallback(vec![
                "GET /host/:host-id",
                "GET /list-all",
                "POST /register-host",
            ])
        })
}
