use ant_data_farm::Dao;
use axum::{extract::State, routing::post, Router};
use axum_extra::routing::RouterExt;
use std::sync::Arc;

async fn expand_ant(State(_dao): State<Arc<Dao>>) -> &'static str {
    "Expanded the ant!!"
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new().route_with_tsr("/expand-ant", post(expand_ant))
}
