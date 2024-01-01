use axum::{extract::State, routing::get, Router};
use axum_extra::routing::RouterExt;

use crate::types::{DbRouter, DbState};

async fn in_progress_deployments(State(_db): DbState) -> &'static str {
    "deployment in progress!"
}

pub fn router() -> DbRouter {
    Router::new().route_with_tsr("/in-progress-deployments", get(in_progress_deployments))
}
