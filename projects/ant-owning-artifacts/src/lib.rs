pub mod client;
pub mod routes;

use ant_data_farm::AntDataFarmClient;
use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::{get, post},
    Router,
};
use axum_extra::routing::RouterExt;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

pub async fn start_server(port: Option<u16>) -> anyhow::Result<(), anyhow::Error> {
    std::env::set_var(
        "RUST_LOG",
        "ant_owning_artifacts=debug,glimmer=debug,tower_http=debug",
    );
    dotenv::dotenv().unwrap();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    info!("Initializing artifact directory...");
    // artifact::initialize()?;

    let db = Arc::new(
        AntDataFarmClient::new(None)
            .await
            .expect("Failed to connect to database!"),
    );

    let api_routes = Router::new()
        .route_with_tsr("/make", post(routes::make::make))
        .with_state(db)
        .fallback(|| async { ant_library::api_fallback(&["POST /make"]) });

    info!("Initializing API routes...");
    let app = Router::new()
        .nest("/api", api_routes)
        .route_with_tsr("/ping", get(ant_library::api_ping))
        .layer(axum::middleware::from_fn(
            ant_library::middleware_print_request_response,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .fallback(|| async { ant_library::api_fallback(&["GET /ping", "/api"]) });

    let port: u16 = port.unwrap_or(4599);
    info!("Starting server on port {port}...");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
