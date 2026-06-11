use std::{net::SocketAddr, path::PathBuf};

use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-archive-storage");

    debug!("Setting up state...");

    let persist_dir = dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR environment variable!");
    let root_dir = PathBuf::from(persist_dir);

    ant_archive_storage::startup_init(&root_dir)
        .await
        .expect("startup init failed");

    let port: u16 = dotenv::var("PORT")
        .expect("PORT environment variable not found")
        .parse()
        .expect("PORT was not u16");

    let metrics_port: u16 = dotenv::var("METRICS_PORT")
        .expect("METRICS_PORT environment variable not found")
        .parse()
        .expect("METRICS_PORT was not u16");

    let (metric_layer, handle) = ant_archive_storage::build_metric_layer();
    let state = ant_archive_storage::AntArchiveStorageState::new(root_dir, handle);

    let metrics_app = ant_archive_storage::make_metrics_routes(state.clone());
    tokio::spawn(async move {
        let addr = SocketAddr::from(([0, 0, 0, 0], metrics_port));
        debug!("Starting metrics server on [{addr}]...");
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect(format!("failed to bind metrics server to {metrics_port}").as_str());
        axum::serve(listener, metrics_app)
            .await
            .expect("metrics server failed");
    });

    let app = ant_archive_storage::make_routes(state, metric_layer).expect("failed to init api");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{addr}]...",
        ant_library::get_mode(),
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}
