use std::{net::SocketAddr, sync::Arc, time::Duration};

use ant_archive_db::AntArchiveDb;
use ant_library::{rng::SystemRng, sd::reader::ServiceDiscovery};
use futures::future::join_all;
use tokio::time::sleep;
use tracing::{debug, error, info};

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

    let state = ant_archive::AntArchiveState {
        chunk_size: 1024 * 1024 * 4, // 4mb
        db,
        sd,
        rng: Arc::new(SystemRng),
        vacuum_grace: chrono::Duration::hours(1),
        instance_id: format!(
            "ant-archive-{}",
            base16ct::lower::encode_string(&rand::random::<[u8; 8]>())
        ),
    };

    let vacuum_state = state.clone();
    let vacuum_handle = tokio::spawn(async move {
        loop {
            // Cheap when nothing has changed, since the vacuum skips itself.
            match ant_archive::routes::vacuum::vacuum(&vacuum_state).await {
                Ok(report) if report.skipped => debug!("Vacuum skipped, no new activity"),
                Ok(report) => info!("Vacuumed: {report:?}"),
                Err(e) => error!("ANT-ERR-149: vacuum failed: {e:?}"),
            }

            sleep(Duration::from_secs(10 * 60)).await;
        }
    });

    let app = ant_archive::make_routes(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{addr}]...",
        ant_library::get_mode()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    let api_handle = tokio::spawn(async {
        axum::serve(listener, app).await.expect("server failed");
    });

    join_all(vec![api_handle, vacuum_handle]).await;
}
