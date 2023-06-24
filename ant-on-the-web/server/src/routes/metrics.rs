use crate::dao::dao::Dao;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

async fn expand_ant(State(dao): State<Arc<Dao>>) -> &'static str {
    "Expanded the ant!!"
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new().route("/expand-ant", post(expand_ant))
}
