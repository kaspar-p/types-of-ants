use crate::dao::{
    dao::Dao,
    daos::hosts::{Host, HostId},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

async fn host(Path(host_id): Path<Uuid>, State(dao): State<Arc<Dao>>) -> impl IntoResponse {
    let host = dao.hosts.get_host_by_id(HostId(host_id));
    if host.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(format!("Host with ID '{}' not found!", host_id)).into_response(),
        );
    } else {
        return (StatusCode::OK, Json(host).into_response());
    }
}

async fn register_host() -> impl IntoResponse {
    (StatusCode::OK, Json("New host registered!"))
}

async fn list_all(State(dao): State<Arc<Dao>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(dao.hosts.get_all_hosts()).into_response(),
    )
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new()
        .route("/host/:host-id", get(host))
        .route("/list-all", get(list_all))
        .route("/register-host", post(register_host))
}
