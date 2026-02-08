use std::{fs::create_dir_all, net::SocketAddr, path::PathBuf, sync::Arc};

use ant_host_agent::client::RemoteAntHostAgentClientFactory;
use ant_library::db::{DatabaseConfig, TypesOfAntsDatabase};
use ant_zookeeper::{dns::CloudFlareDns, state::AntZookeeperState};
use ant_zookeeper_db::AntZooStorageClient;
use anyhow::Context;
use rsa::rand_core::OsRng;
use tokio::{signal, sync::Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};
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

        db: AntZooStorageClient::connect(&DatabaseConfig {
            port: std::env::var("ANT_ZOOKEEPER_DB_PORT")
                .context("ANT_ZOOKEEPER_DB_PORT")?
                .parse()?,
            database_name: ant_library::secret::load_secret("ant_zookeeper_db_db")?,
            database_password: ant_library::secret::load_secret("ant_zookeeper_db_password")?,
            database_user: ant_library::secret::load_secret("ant_zookeeper_db_user")?,
            host: std::env::var("ANT_ZOOKEEPER_DB_HOST")
                .context("ANT_ZOOKEEPER_DB_HOST")?
                .parse()?,
            migration_dirs: vec![],
        })
        .await?,

        rng: OsRng,

        dns: Arc::new(Mutex::new(CloudFlareDns::new(
            ant_library::secret::load_secret("cloudflare")?,
            ant_library::secret::load_secret("cloudflare_zone_id")?,
        ))),
        acme_url: acme_lib::DirectoryUrl::LetsEncrypt,
        acme_contact_email: std::env::var("ANT_ZOOKEEPER_ACME_CONTACT_EMAIL")
            .context("ANT_ZOOKEEPER_ACME_CONTACT_EMAIL")?,

        ant_host_agent_factory: Arc::new(Mutex::new(RemoteAntHostAgentClientFactory)),
    };

    let app = ant_zookeeper::make_routes(state)?;

    // Setup the iteration thread, to slowly progress the pipeline
    let port: u16 = dotenv::var("ANT_ZOOKEEPER_PORT")
        .context("ANT_ZOOKEEPER_PORT")?
        .parse()?;

    let port2 = port.clone();
    let scheduler = JobScheduler::new().await.unwrap();
    scheduler
        .add(
            Job::new_async("every 2 seconds", move |_, _| {
                Box::pin(async move {
                    reqwest::Client::new()
                        .post(format!("http://localhost:{}/deployment/iteration", port2))
                        .send()
                        .await
                        .unwrap()
                        .error_for_status()
                        .unwrap();
                })
            })
            .expect("job creation"),
        )
        .await
        .expect("deployment iteration job");

    scheduler.shutdown_on_ctrl_c();

    scheduler.start().await.expect("start schedular");

    let addr = SocketAddr::from(([0, 0, 0, 0], port.clone()));
    debug!(
        "Starting [{}] server on [{}]...",
        ant_library::get_mode(),
        addr.to_string()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server failed");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { println!("shutdown_signal");},
        _ = terminate => {println!("terminating");},
    }
}
