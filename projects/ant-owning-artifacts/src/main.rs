use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::debug;

use ant_owning_artifacts::{artifact, procs::deploy_project::deploy_project};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    std::env::set_var(
        "RUST_LOG",
        "ant_on_the_web=debug,glimmer=debug,tower_http=debug",
    );
    dotenv::dotenv().unwrap();

    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    debug!("Creating artifact directory!");
    artifact::initialize()?;

    debug!("Initializing API routes...");
    let app = Router::new()
        .route("/DeployProject", post(deploy_project))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    debug!("Starting server...");

    let port = dotenv::var("HOST_AGENT_PORT").unwrap_or(String::from("4499"));
    debug!("Starting host agent on port {port}");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3499));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
