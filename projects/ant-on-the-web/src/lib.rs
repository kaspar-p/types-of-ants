use ant_library::routes::Routes;
use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::routing::RouterExt;
use http::{header, Response, StatusCode};
use hyper::http::Method;
use routes::{lib::err::AntOnTheWebError, version};
use std::sync::Arc;
use throttle::ThrottleExtractor;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

mod middleware;
use tower_governor::{governor::GovernorConfigBuilder, GovernorError, GovernorLayer};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
};
use tracing::debug;

mod clients;
mod routes;
pub mod state;
mod throttle;

pub use crate::clients::email;
pub use crate::clients::sms;

pub use crate::routes::ants;
pub use crate::routes::api_tokens;
pub use crate::routes::deployments;
pub use crate::routes::hosts;
pub use crate::routes::lib::err;
use crate::routes::lib::telemetry::telemetry_cookie_middleware;
pub use crate::routes::lib::two_factor;
pub use crate::routes::metrics;
pub use crate::routes::prints;
pub use crate::routes::tests;
pub use crate::routes::users;
pub use crate::routes::web_actions;
pub use crate::routes::webhooks;

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
            AntOnTheWebError::InternalServerError { id: "ANT-ERR-123", err: Some(anyhow::Error::msg(format!("{:?}", err))) }
                .into_response()
        }
    }
}

pub struct ApiOptions {
    pub tps: u32,
}

pub fn make_routes(
    state: &state::InnerApiState,
    opts: ApiOptions,
) -> Result<Router, anyhow::Error> {
    debug!("Initializing API routes...");

    let throttling = Arc::new(
        GovernorConfigBuilder::default()
            .period(std::time::Duration::from_millis(
                (1000 / opts.tps as u64).max(1),
            ))
            .burst_size(opts.tps)
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

    // Public routes: logging applied here.
    // Sensitive routes (users, webhooks): carry their own logging + redaction layers
    // with the correct ordering (redaction outermost so it runs before logging).
    let api_routes = Routes::new()
        .merge_routes(version::routes())
        .nest_routes("/ants", ants::routes())
        // .nest("/msg", routes::msg::router())
        .nest_routes("/api-tokens", api_tokens::routes())
        .nest_routes("/hosts", hosts::routes())
        .nest_routes("/web-actions", web_actions::routes())
        .nest_routes("/prints", prints::routes())
        // .nest("/tests", tests::router())
        // .nest("/metrics", metrics::router())
        // .nest("/deployments", deployments::router())
        .layer(axum::middleware::from_fn(
            ant_library::middleware::print_request_response,
        ))
        .merge_routes(
            Routes::new()
                .nest_routes("/users", users::routes())
                .nest_routes("/webhooks", webhooks::routes()),
        )
        .build()
        .with_state(state.clone());

    debug!("Initializing site routes...");
    let app = Router::new()
        .nest("/api", api_routes)
        .route_with_tsr("/ping", get(ant_library::api_ping))
        // Marking the main filesystem as fallback allows wrong paths like
        // /api/something to still hit the /api router fallback()
        .fallback_service(ServeDir::new(state.static_dir.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(cors)
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(GovernorLayer { config: throttling })
                .layer(CookieManagerLayer::new())
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    telemetry_cookie_middleware,
                ))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::x_ant_middleware,
                )),
        );

    return Ok(app);
}
