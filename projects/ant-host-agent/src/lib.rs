pub mod clients;
pub mod routes;
pub mod state;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::get,
    Router,
};
use axum_extra::routing::RouterExt;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, trace::TraceLayer};

use crate::state::AntHostAgentState;

pub async fn make_routes(state: AntHostAgentState) -> Result<Router, anyhow::Error> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    let api: Router = Router::new()
        .nest("/service", crate::routes::service::make_routes())
        .route_with_tsr(
            "/ping",
            get(ant_library::api_ping).post(ant_library::api_ping),
        )
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(CatchPanicLayer::custom(ant_library::middleware_catch_panic))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware_print_request_response,
                ))),
        )
        .fallback(|| async { ant_library::api_fallback(&["GET|POST /ping", "/service"]) });

    return Ok(api);
}
