use ant_data_farm::Dao;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use axum_extra::routing::RouterExt;
use std::sync::Arc;

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
    Router::new().route_with_tsr("/host-status/:host-id-or-name", get(host_status))
}
