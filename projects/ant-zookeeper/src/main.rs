use std::{fs::create_dir_all, net::SocketAddr, path::PathBuf, sync::Arc};

use ant_library::db::{DatabaseConfig, TypesOfAntsDatabase};
use ant_zoo_storage::AntZooStorageClient;
use ant_zookeeper::{dns::CloudFlareDns, state::AntZookeeperState};
use rsa::rand_core::OsRng;
use tokio::sync::Mutex;
use tracing::debug;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    ant_library::set_global_logs("ant-zookeeper");

    debug!("Setting up state...");

    let persist_dir = dotenv::var("PERSIST_DIR")?;
    let root_path = dotenv::var("ANT_ZOOKEEPER_ROOT_PATH")?;

    let root_dir = PathBuf::from(persist_dir).join(root_path);
    create_dir_all(&root_dir)?;

    let state = AntZookeeperState {
        root_dir,

        db: AntZooStorageClient::connect(&DatabaseConfig {
            port: dotenv::var("ANT_ZOOKEEPER_PORT")?.parse()?,
            database_name: ant_library::secret::load_secret("ant_zoo_storage_db")?,
            database_password: ant_library::secret::load_secret("ant_zoo_storage_password")?,
            database_user: ant_library::secret::load_secret("ant_zoo_storage_user")?,
            host: dotenv::var("ANT_ZOOKEEPER_HOST")?.parse()?,
            migration_dir: None,
        })
        .await?,

        rng: OsRng,

        dns: Arc::new(Mutex::new(CloudFlareDns::new(
            ant_library::secret::load_secret("cloudflare").unwrap(),
            ant_library::secret::load_secret("cloudflare_zone_id").unwrap(),
        ))),
        acme_url: acme_lib::DirectoryUrl::LetsEncrypt,
        acme_contact_email: dotenv::var("ANT_ZOOKEEPER_ACME_CONTACT_EMAIL")
            .expect("No ANT_ZOOKEEPER_ACME_CONTACT_EMAIL variable."),
    };

    let app = ant_zookeeper::make_routes(state).expect("failed to init api");

    let port: u16 = dotenv::var("ANT_ZOOKEEPER_PORT")
        .expect("ANT_ZOOKEEPER_PORT environment variable not found")
        .parse()
        .expect("ANT_ZOOKEEPER_PORT was not u16");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{}]...",
        ant_library::get_mode(),
        addr.to_string()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");

    Ok(())
}
