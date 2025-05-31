use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::routing::RouterExt;
use http::{header, Response, StatusCode};
use hyper::http::Method;
use std::sync::Arc;
use throttle::ThrottleExtractor;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorError, GovernorLayer};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::debug;

mod clients;
mod routes;
mod throttle;
pub mod types;

pub use crate::clients::sms;
use crate::err::AntOnTheWebError;
pub use crate::routes::ants;
pub use crate::routes::deployments;
pub use crate::routes::hosts;
pub use crate::routes::lib::err;
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

fn handle_throttling_error(err: &GovernorError) -> Response<axum::body::Body> {
    match err {
        GovernorError::TooManyRequests {
            wait_time: _,
            headers: _,
        } => (StatusCode::TOO_MANY_REQUESTS, "Throttling limit reached.").into_response(),
        err => {
            AntOnTheWebError::InternalServerError(Some(anyhow::Error::msg(format!("{:?}", err))))
                .into_response()
        }
    }
}

pub fn make_routes(state: types::InnerApiState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API routes...");

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            // 10 TPS
            .period(std::time::Duration::from_millis(100))
            .burst_size(25)
            .use_headers()
            .key_extractor(ThrottleExtractor::new()) // Limit based on X-Real-IP Header
            .error_handler(|err| handle_throttling_error(&err))
            .finish()
            .unwrap(),
    );

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(origins())
        .allow_credentials(true)
        .allow_headers([header::CONTENT_TYPE]);

    let api_routes = Router::new()
        .nest("/ants", ants::router())
        // .nest("/msg", routes::msg::router())
        .nest("/users", users::router())
        .nest("/hosts", hosts::router())
        // .nest("/tests", tests::router())
        // .nest("/metrics", metrics::router())
        // .nest("/deployments", deployments::router())
        .with_state(state)
        .layer(axum::middleware::from_fn(
            ant_library::middleware_print_request_response,
        ))
        .layer(CatchPanicLayer::custom(ant_library::middleware_catch_panic))
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
                .layer(cors)
                .layer(GovernorLayer {
                    config: governor_conf,
                }),
        );

    return Ok(app);
}
