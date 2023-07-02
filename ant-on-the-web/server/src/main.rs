mod middleware;
mod routes;
mod types;

use ant_data_farm::connect;
use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::get,
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{debug, info};

#[tokio::main]
async fn main() {
    std::env::set_var(
        "RUST_LOG",
        "ant_on_the_web=debug,glimmer=debug,tower_http=debug",
    );

    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    debug!("Setting up database connection pool...");
    let dao = Arc::new(connect().await);

    debug!("Initializing API routes...");
    let api_routes = Router::new()
        .nest("/ants", routes::ants::router())
        .nest("/users", routes::users::router())
        .nest("/hosts", routes::hosts::router())
        .nest("/tests", routes::tests::router())
        .nest("/metrics", routes::metrics::router())
        .nest("/deployments", routes::deployments::router())
        .with_state(dao)
        .layer(axum::middleware::from_fn(
            middleware::print_request_response,
        ))
        .fallback(|| async {
            middleware::fallback(&[
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
        .route("/ping", get(|| async { ping() }))
        .nest("/api", api_routes)
        // Marking the main filesystem as fallback allows wrong paths like
        // /api/something to still hit the /api router fallback()
        .fallback_service(ServeDir::new("static"))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    debug!("Starting server...");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3499));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn ping() -> &'static str {
    info!("Got ping, responding with pong!");
    "pong"
}
