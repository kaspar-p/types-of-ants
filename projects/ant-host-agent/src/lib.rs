pub mod client;
mod err;
pub mod routes;
pub mod state;
pub mod systemd;

use ant_library::routes::Routes;
use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};

use crate::state::AntHostAgentState;

pub fn make_routes(state: AntHostAgentState) -> Result<Router, anyhow::Error> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    let api: Router = Routes::new()
        .nest_routes("/service", crate::routes::service::routes())
        .get("/ping", get(ant_library::api_ping))
        .post("/ping", post(ant_library::api_ping))
        .build()
        .with_state(state)
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

    return Ok(api);
}
