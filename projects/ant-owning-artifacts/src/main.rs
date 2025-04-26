mod routes;

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

use ant_owning_artifacts::start_server;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    start_server(None).await?;

    Ok(())
}
