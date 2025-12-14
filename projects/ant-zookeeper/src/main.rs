use std::{fs::create_dir_all, net::SocketAddr, path::PathBuf, sync::Arc};

use ant_library::db::{DatabaseConfig, TypesOfAntsDatabase};
use ant_zoo_storage::AntZooStorageClient;
use ant_zookeeper::{dns::CloudFlareDns, state::AntZookeeperState};
use anyhow::Context;
use rsa::rand_core::OsRng;
use tokio::sync::Mutex;
use tracing::debug;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    ant_library::set_global_logs("ant-zookeeper");

    debug!("Setting up state...");

    let persist_dir = std::env::var("PERSIST_DIR").context("PERSIST_DIR")?;
    let root_path = std::env::var("ANT_ZOOKEEPER_ROOT_PATH").context("ANT_ZOOKEEPER_ROOT_PATH")?;

    let root_dir = PathBuf::from(persist_dir).join(root_path);
    create_dir_all(&root_dir)?;

    let state = AntZookeeperState {
        root_dir,

        db: Arc::new(Mutex::new(
            AntZooStorageClient::connect(&DatabaseConfig {
                port: std::env::var("ANT_ZOO_STORAGE_PORT")
                    .context("ANT_ZOO_STORAGE_PORT")?
                    .parse()?,
                database_name: ant_library::secret::load_secret("ant_zoo_storage_db")?,
                database_password: ant_library::secret::load_secret("ant_zoo_storage_password")?,
                database_user: ant_library::secret::load_secret("ant_zoo_storage_user")?,
                host: std::env::var("ANT_ZOO_STORAGE_HOST")
                    .context("ANT_ZOO_STORAGE_HOST")?
                    .parse()?,
                migration_dir: None,
            })
            .await?,
        )),

        rng: OsRng,

        dns: Arc::new(Mutex::new(CloudFlareDns::new(
            ant_library::secret::load_secret("cloudflare")?,
            ant_library::secret::load_secret("cloudflare_zone_id")?,
        ))),
        acme_url: acme_lib::DirectoryUrl::LetsEncrypt,
        acme_contact_email: std::env::var("ANT_ZOOKEEPER_ACME_CONTACT_EMAIL")
            .context("ANT_ZOOKEEPER_ACME_CONTACT_EMAIL")?,
    };

    let app = ant_zookeeper::make_routes(state)?;

    let port: u16 = dotenv::var("ANT_ZOOKEEPER_PORT")
        .context("ANT_ZOOKEEPER_PORT")?
        .parse()?;
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
