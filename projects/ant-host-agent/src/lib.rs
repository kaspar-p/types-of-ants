pub mod clients;
mod common;

pub use common::{get_project_logs, kill_project, launch_project};

mod procs;
mod routes;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::get,
    Router,
};
use axum_extra::routing::RouterExt;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

pub async fn start_server(port: Option<u16>) -> Result<(), anyhow::Error> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    let routes = Router::new().fallback(|| async { ant_library::api_fallback(&[]) });
    // .route_with_tsr("/describe_projects", get(describe_projects))
    // .route_with_tsr("/kill_project", post(kill_project))
    // .route_with_tsr("/launch_project", post(launch_project))
    // .layer(DefaultBodyLimit::disable())
    // .layer(RequestBodyLimitLayer::new(
    // 100 * 1024 * 1024, /* 100mb */
    // ))

    let app = Router::new()
        .nest("/api", routes)
        .route_with_tsr(
            "/ping",
            get(ant_library::api_ping).post(ant_library::api_ping),
        )
        .layer(axum::middleware::from_fn(
            ant_library::middleware_print_request_response,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .fallback(|| async { ant_library::api_fallback(&["GET|POST /ping", "/api"]) });

    info!("Starting server...");
    let port = port.unwrap_or(
        dotenv::var("ANT_HOST_AGENT_PORT")
            .expect("Could not find ANT_HOST_AGENT_PORT environment variable")
            .parse::<u16>()
            .expect("ANT_HOST_AGENT_PORT environment variable needs to be a valid port!"),
    );
    info!("Starting host agent on port {port}");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind to {port}").as_str());
    axum::serve(listener, app).await.unwrap();

    return Ok(());
}
