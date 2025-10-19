use std::time::Duration;

use axum::{response::IntoResponse, routing::post, Json, Router};
use http::{header, Method};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::debug;

use crate::state::AntZookeeperState;

pub mod state;

#[derive(Serialize, Deserialize)]
pub struct DeployServiceRequest {
    pub project: String,
    pub version: String,
}

pub fn make_routes(s: AntZookeeperState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Router::new()
        // .route("/enable-service", post(enable_service))
        .with_state(s)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(CatchPanicLayer::custom(ant_library::middleware_catch_panic))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware_print_request_response,
                ))),
        );

    return Ok(app);
}
