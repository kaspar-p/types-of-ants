use ant_library::routes::Routes;
use axum::{response::IntoResponse, routing::get};

use crate::state::ApiRoutes;

async fn current_version() -> impl IntoResponse {
    ant_library::manifest_file::read_local_manifest_file(None).commit_number
}

pub fn routes() -> ApiRoutes {
    Routes::new().get("/version", get(current_version))
}
