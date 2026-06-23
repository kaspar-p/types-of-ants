use ant_library::routes::Routes;
use axum::Router;
use http::{header, Method};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
};
use tracing::debug;

use crate::state::AntZookeeperState;

pub mod client;
pub mod dns;
pub mod err;
pub mod event_loop;
mod fs;
pub mod pipeline;
pub mod pipeline_engine;
pub mod routes;
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
        .allow_headers([header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::any());

    debug!("Initializing site routes...");
    let app = Routes::new()
        .nest_routes("/pipeline", routes::pipeline::routes())
        .nest_routes("/deployment", routes::deployment::routes())
        .nest_routes("/service", routes::service::routes())
        .nest_routes("/cert", routes::cert::routes())
        .nest_routes("/projects", routes::projects::routes())
        .build()
        .with_state(s)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(cors)
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                ))),
        );

    return Ok(app);
}
