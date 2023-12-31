use axum::{extract::State, routing::post, Router};
use axum_extra::routing::RouterExt;

use crate::types::{DbRouter, DbState};

async fn expand_ant(State(_db): DbState) -> &'static str {
    "Expanded the ant!!"
}

pub fn router() -> DbRouter {
    Router::new().route_with_tsr("/expand-ant", post(expand_ant))
}
