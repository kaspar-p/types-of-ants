mod common;
mod procs;
mod routes;

use axum::{
    extract::DefaultBodyLimit,
    http::{header::CONTENT_TYPE, Method},
    routing::{get, post},
    Router,
};
use routes::{describe_projects, kill_project, launch_project, ping};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::debug;

#[tokio::main]
async fn main() {
    std::env::set_var(
        "RUST_LOG",
        "ant_on_the_web=debug,glimmer=debug,tower_http=debug",
    );
    dotenv::dotenv().expect("No .env file found!");

    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    debug!("Initializing API routes...");
    let app = Router::new()
        .route("/ping", get(ping).post(ping))
        .route("/describe_projects", get(describe_projects))
        .route("/kill_project", post(kill_project))
        .route("/launch_project", post(launch_project))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            100 * 1024 * 1024, /* 100mb */
        ))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    debug!("Starting server...");
    let port = dotenv::var("HOST_AGENT_PORT")
        .unwrap_or(String::from("4499"))
        .parse::<u16>()
        .expect("HOST_AGENT_PORT environment variable needs to be a valid port!");
    debug!("Starting host agent on port {port}");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
