use crate::dao::dao::Dao;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

async fn host_status(
    Path(host_name): Path<String>,
    State(dao): State<Arc<Dao>>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(format!(
            "The host {} is probably down, I'm not very good at any of this.",
            host_name
        )),
    )
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new().route("/host-status/:host-id-or-name", get(host_status))
}
