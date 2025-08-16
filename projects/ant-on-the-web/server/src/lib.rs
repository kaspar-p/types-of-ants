use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::routing::RouterExt;
use http::{header, Response, StatusCode};
use hyper::http::Method;
use routes::{lib::err::AntOnTheWebError, version};
use std::sync::Arc;
use throttle::ThrottleExtractor;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
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
pub mod state;
mod throttle;

pub use crate::clients::email;
pub use crate::clients::sms;

pub use crate::routes::ants;
pub use crate::routes::deployments;
pub use crate::routes::hosts;
pub use crate::routes::lib::err;
use crate::routes::lib::telemetry::telemetry_cookie_middleware;
pub use crate::routes::lib::two_factor;
pub use crate::routes::metrics;
pub use crate::routes::tests;
pub use crate::routes::users;
pub use crate::routes::web_actions;

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

pub fn make_routes(state: &state::InnerApiState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API routes...");

    let throttling = Arc::new(
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
        .merge(version::router())
        .nest("/ants", ants::router())
        // .nest("/msg", routes::msg::router())
        .nest("/users", users::router())
        .nest("/hosts", hosts::router())
        .nest("/web-actions", web_actions::router())
        // .nest("/tests", tests::router())
        // .nest("/metrics", metrics::router())
        // .nest("/deployments", deployments::router())
        .with_state(state.clone())
        .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
            ant_library::middleware_print_request_response,
        )))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "GET /version",
                "nested: /ants",
                "nested: /users",
                "nested: /hosts",
                "nested: /tests",
                "nested: /metrics",
                "nested: /deployments",
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
                .layer(CatchPanicLayer::custom(ant_library::middleware_catch_panic))
                .layer(GovernorLayer { config: throttling })
                .layer(CookieManagerLayer::new())
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    telemetry_cookie_middleware,
                )),
        );

    return Ok(app);
}
