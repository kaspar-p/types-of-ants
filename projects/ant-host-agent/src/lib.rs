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
use tracing::{debug, Level};
use tracing_subscriber::FmtSubscriber;

pub async fn start_server(port: Option<u16>) -> Result<(), anyhow::Error> {
    std::env::set_var(
        "RUST_LOG",
        "ant_host_agent=debug,glimmer=debug,tower_http=debug",
    );
    dotenv::dotenv().expect("No .env file found!");

    // initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_ansi(false)
        .with_writer(tracing_appender::rolling::hourly(
            "./logs",
            "ant-host-agent.log",
        ))
        .finish();
    let _ = tracing::subscriber::set_default(subscriber);

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

    debug!("Initializing API routes...");
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

    debug!("Starting server...");
    let port = port.unwrap_or(
        dotenv::var("HOST_AGENT_PORT")
            .expect("Could not find HOST_AGENT_PORT environment variable")
            .parse::<u16>()
            .expect("HOST_AGENT_PORT environment variable needs to be a valid port!"),
    );
    debug!("Starting host agent on port {port}");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    return Ok(());
}
