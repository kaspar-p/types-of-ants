use std::{net::SocketAddr, sync::Arc};

use ant_archive_db::AntArchiveDb;
use ant_library::{rng::SystemRng, sd::reader::ServiceDiscovery};
use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-archive");

    debug!("Setting up state...");

    let port: u16 = dotenv::var("PORT")
        .expect("PORT not set")
        .parse()
        .expect("PORT was not u16");

    let matchmaker_port: u16 = dotenv::var("ANT_MATCHMAKER_HTTP_PORT")
        .expect("ANT_MATCHMAKER_HTTP_PORT not set")
        .parse()
        .expect("ANT_MATCHMAKER_HTTP_PORT was not u16");

    let sd = Arc::new(ServiceDiscovery::new(matchmaker_port));

    let db = AntArchiveDb::connect_discovered(sd.clone())
        .await
        .expect("failed to connect to ant-archive-db");

    let state = ant_archive::AntArchiveState { db, sd, rng: Arc::new(SystemRng) };

    let app = ant_archive::make_routes(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{addr}]...",
        ant_library::get_mode()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}
