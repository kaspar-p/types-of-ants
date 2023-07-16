use ant_data_farm::Dao;
use axum::{extract::State, routing::get, Router};
use axum_extra::routing::RouterExt;
use std::sync::Arc;

async fn in_progress_deployments(State(_dao): State<Arc<Dao>>) -> &'static str {
    "deployment in progress!"
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new().route_with_tsr("/in-progress-deployments", get(in_progress_deployments))
}
