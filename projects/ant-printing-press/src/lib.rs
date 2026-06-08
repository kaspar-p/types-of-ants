use ant_library::routes::Routes;
use axum::Router;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tracing::debug;

use crate::state::AntPrintingPressState;

mod err;
pub mod routes;
pub mod state;

pub fn make_routes(state: &AntPrintingPressState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API routes...");

    let app = Routes::new()
        .nest_routes("/print", routes::print::routes())
        .build()
        .with_state(state.clone())
        .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
            ant_library::middleware::print_request_response,
        )))
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                )),
        );

    return Ok(app);
}
