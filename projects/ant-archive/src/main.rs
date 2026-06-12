use std::{net::SocketAddr, sync::Arc};

use ant_archive_db::AntArchiveDb;
use ant_library::sd::reader::ServiceDiscovery;
use tracing::{debug, warn};

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

    let kek = load_kek().expect("failed to load ant_archive_kek secret");

    let sd = Arc::new(ServiceDiscovery::new(matchmaker_port));

    let db = AntArchiveDb::connect_discovered(sd.clone())
        .await
        .expect("failed to connect to ant-archive-db");

    let kek_id = db
        .get_active_kek_id()
        .await
        .expect("failed to query kek")
        .expect("no active KEK version found in database");

    let storage_nodes = discover_storage_nodes(&sd, &db).await;

    let state = ant_archive::AntArchiveState::new(db, storage_nodes, kek_id, kek);

    let app = ant_archive::make_routes(state).expect("failed to init api");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!("Starting [{}] server on [{addr}]...", ant_library::get_mode());
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}

fn load_kek() -> Result<[u8; 32], anyhow::Error> {
    let hex = ant_library::secret::load_secret("ant_archive_kek")?;
    let bytes = hex::decode(hex.trim())?;
    let kek: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("ant_archive_kek must decode to exactly 32 bytes"))?;
    Ok(kek)
}

async fn discover_storage_nodes(
    sd: &ServiceDiscovery,
    db: &AntArchiveDb,
) -> Vec<ant_archive::AntArchiveStorageNodeClient> {
    let password = ant_library::secret::load_secret("ant_archive_storage_client_password")
        .expect("ant_archive_storage_client_password secret not found");

    let endpoints = sd.resolve_all("ant-archive-storage").await;
    let mut clients = Vec::new();

    for ep in &endpoints {
        match db.get_storage_node_by_node_name(&ep.node).await {
            Ok(Some(node_id)) => {
                let client = ant_archive::AntArchiveStorageNodeClient::new(
                    node_id,
                    format!("http://{}:{}", ep.address, ep.port),
                    "user",
                    &password,
                );
                clients.push(client);
            }
            Ok(None) => {
                warn!(
                    "Consul returned ant-archive-storage on node '{}' but no matching archive_storage_node row — skipping",
                    ep.node
                );
            }
            Err(e) => {
                warn!("Failed to look up storage node '{}': {e}", ep.node);
            }
        }
    }

    assert!(
        !clients.is_empty(),
        "no registered ant-archive-storage nodes found — ensure storage nodes are running and registered"
    );

    clients
}
