use ant_data_farm::AntDataFarmClient;
use axum::{routing::get, Router};
use axum_extra::routing::RouterExt;
use hyper::http::{header::CONTENT_TYPE, Method};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::debug;

mod clients;
mod routes;
mod types;

pub use crate::routes::users;

pub fn make_routes(ant_data_farm_client: Arc<AntDataFarmClient>) -> Result<Router, anyhow::Error> {
    let origins_string: String = dotenv::var("ANT_ON_THE_WEB_ALLOWED_ORIGINS")?;
    let origins = origins_string.split(",");
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(
            origins
                .map(|fqdn| fqdn.parse().expect("fqdn valid"))
                .collect::<Vec<_>>(),
        )
        .allow_headers([CONTENT_TYPE]);

    debug!("Initializing API routes...");
    let api_routes = Router::new()
        .nest("/ants", routes::ants::router())
        // .nest("/msg", routes::msg::router())
        .nest("/users", routes::users::router())
        .nest("/hosts", routes::hosts::router())
        .nest("/tests", routes::tests::router())
        .nest("/metrics", routes::metrics::router())
        .nest("/deployments", routes::deployments::router())
        .with_state(ant_data_farm_client)
        .layer(axum::middleware::from_fn(
            ant_library::middleware_print_request_response,
        ))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "/ants",
                "/users",
                "/hosts",
                "/tests",
                "/metrics",
                "/deployments",
            ])
        });

    debug!("Initializing site routes...");
    let app = Router::new()
        .nest("/api", api_routes)
        .route_with_tsr("/ping", get(ant_library::api_ping))
        // Marking the main filesystem as fallback allows wrong paths like
        // /api/something to still hit the /api router fallback()
        .fallback_service(ServeDir::new("static"))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    return Ok(app);
}
