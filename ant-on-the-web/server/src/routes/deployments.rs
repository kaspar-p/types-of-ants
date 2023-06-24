use crate::dao::dao::Dao;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

async fn in_progress_deployments(State(dao): State<Arc<Dao>>) -> &'static str {
    return "deployment in progress!";
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new().route("/in-progress-deployments", get(in_progress_deployments))
}
