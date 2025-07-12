use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::routing::RouterExt;

use crate::state::ApiRouter;

async fn current_version() -> impl IntoResponse {
    ant_library::manifest_file::read_local_manifest_file().commit_number
}

pub fn router() -> ApiRouter {
    Router::new()
        .route_with_tsr("/version", get(current_version))
        .fallback(|| async { ant_library::api_fallback(&["GET /version"]) })
}
