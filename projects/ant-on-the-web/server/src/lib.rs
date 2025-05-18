use ant_data_farm::AntDataFarmClient;
use axum::{routing::get, Router};
use axum_extra::routing::RouterExt;
use http::header;
use hyper::http::{header::CONTENT_TYPE, Method};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::AllowOrigin;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::debug;

mod clients;
mod routes;
mod types;

pub use crate::routes::ants;
pub use crate::routes::deployments;
pub use crate::routes::hosts;
pub use crate::routes::metrics;
pub use crate::routes::tests;
pub use crate::routes::users;

fn origins() -> AllowOrigin {
    match dotenv::var("ANT_ON_THE_WEB_ALLOWED_ORIGINS") {
        // Block all
        Err(_) => AllowOrigin::predicate(|_, _| false),
        // Allow all
        Ok(val) if val.as_str() == "*" => AllowOrigin::any(),
        // Comma-separated string
        Ok(origins_string) => {
            let origins = origins_string.split(",");
            AllowOrigin::list(
                origins
                    .map(|fqdn| fqdn.parse().expect("fqdn valid"))
                    .collect::<Vec<_>>(),
            )
        }
    }
}

pub fn make_routes(ant_data_farm_client: Arc<AntDataFarmClient>) -> Result<Router, anyhow::Error> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(origins())
        .allow_credentials(true)
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing API routes...");
    let api_routes = Router::new()
        .nest("/ants", ants::router())
        // .nest("/msg", routes::msg::router())
        .nest("/users", users::router())
        .nest("/hosts", hosts::router())
        .nest("/tests", tests::router())
        .nest("/metrics", metrics::router())
        .nest("/deployments", deployments::router())
        .with_state(ant_data_farm_client)
        .layer(axum::middleware::from_fn(
            ant_library::middleware_print_request_response,
        ))
        // .layer(axum::middleware::from_fn(
        //     ant_library::middleware_mode_headers,
        // ))
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
