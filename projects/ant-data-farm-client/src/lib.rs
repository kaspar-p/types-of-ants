mod dao;
mod types;

pub use crate::dao::dao_trait::DaoTrait;
pub use crate::dao::daos::ants;
use crate::dao::daos::api_tokens::ApiTokensDao;
pub use crate::dao::daos::hosts;
pub use crate::dao::daos::releases;
pub use crate::dao::daos::tweets;
pub use crate::dao::daos::users;
pub use crate::dao::daos::verifications;
pub use crate::dao::daos::web_actions;

use crate::{
    dao::daos::{
        ants::AntsDao, hosts::HostsDao, releases::ReleasesDao, tweets::TweetsDao, users::UsersDao,
        verifications::VerificationsDao, web_actions::WebActionsDao,
    },
    dao::db::Database,
    types::ConnectionPool,
};
use bb8::Pool;
use bb8::PooledConnection;
use bb8_postgres::PostgresConnectionManager;
use std::fs::read_dir;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_postgres::NoTls;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct DatabaseCredentials {
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// The database port.
    /// If omitted, reads ANT_DATA_FARM_PORT environment variable.
    pub port: Option<u16>,
    /// The credentials to connect to the database.
    /// If omitted, tries to get credentials from ant_library::secret::load_secret and the local fs.
    pub creds: Option<DatabaseCredentials>,
    /// The IP-address host of the database.
    /// If omitted, reads ANT_DATA_FARM_HOST environment variable.
    pub host: Option<String>,
    /// On client startup, execute the SQL within the directory to bootstrap schemas and databases.
    pub migration_dir: Option<PathBuf>,
}

fn get_credentials_from_env() -> Result<DatabaseCredentials, anyhow::Error> {
    Ok(DatabaseCredentials {
        database_name: ant_library::secret::load_secret("postgres_db")?,
        database_user: ant_library::secret::load_secret("postgres_user")?,
        database_password: ant_library::secret::load_secret("postgres_password")?,
    })
}

fn get_port_from_env() -> Option<u16> {
    dotenv::dotenv().ok()?;
    dotenv::var("ANT_DATA_FARM_PORT").ok()?.parse::<u16>().ok()
}

fn get_host_from_env() -> Option<String> {
    dotenv::dotenv().ok()?;
    dotenv::var("ANT_DATA_FARM_HOST").ok()
}

fn make_connection_string(
    username: &str,
    password: &str,
    host: &str,
    port: u16,
    db_name: &str,
) -> String {
    format!("postgresql://{username}:{password}@{host}:{port}/{db_name}")
}

async fn database_connection(
    config: &DatabaseConfig,
) -> Result<Pool<PostgresConnectionManager<NoTls>>, dotenv::Error> {
    let port = config
        .port
        .unwrap_or_else(|| get_port_from_env().expect("db: port not in environment"))
        .clone();
    let db_creds = config
        .creds
        .clone()
        .unwrap_or_else(|| get_credentials_from_env().expect("db: credentials not in environment"));

    let host = config
        .host
        .clone()
        .unwrap_or_else(|| get_host_from_env().expect("db: host not in environment"));

    let connection_string = make_connection_string(
        &db_creds.database_user,
        &db_creds.database_password,
        &host,
        port,
        &db_creds.database_name,
    );

    debug!(
        "Connecting to database {}",
        make_connection_string(
            "[redacted]",
            "[redacted]",
            &host,
            port,
            &db_creds.database_name
        )
    );
    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> = Pool::builder()
        .build(manager)
        .await
        .expect("db: connection failed");

    Ok(pool)
}

pub struct AntDataFarmClient {
    pub ants: RwLock<AntsDao>,
    pub releases: RwLock<ReleasesDao>,
    pub users: RwLock<UsersDao>,
    pub api_tokens: RwLock<ApiTokensDao>,
    pub verifications: RwLock<VerificationsDao>,
    pub tweets: RwLock<TweetsDao>,
    pub hosts: RwLock<HostsDao>,
    pub web_actions: RwLock<WebActionsDao>,
    // pub deployments: DeploymentsDao,
    // pub metrics: MetricsDao,
    // pub tests: TestsDao,
}

impl AntDataFarmClient {
    pub async fn new(config: Option<DatabaseConfig>) -> Result<AntDataFarmClient, anyhow::Error> {
        let config = match config {
            None => DatabaseConfig {
                port: None,
                creds: None,
                host: None,
                migration_dir: None,
            },
            Some(c) => c,
        };

        let pool = database_connection(&config).await?;

        if let Some(migration_dir) = &config.migration_dir {
            let con = pool.get().await?;
            bootstrap(con, migration_dir).await?;
        }

        info!("Initializing data access layer...");
        let client = AntDataFarmClient::initialize(pool).await?;
        return Ok(client);
    }

    async fn initialize(pool: ConnectionPool) -> Result<AntDataFarmClient, anyhow::Error> {
        let database: Arc<Mutex<Database>> = Arc::new(Mutex::new(pool));

        Ok(AntDataFarmClient {
            ants: RwLock::new(AntsDao::new(database.clone()).await?),
            api_tokens: RwLock::new(ApiTokensDao::new(database.clone()).await?),
            releases: RwLock::new(ReleasesDao::new(database.clone()).await),
            tweets: RwLock::new(TweetsDao::new(database.clone())),
            users: RwLock::new(UsersDao::new(database.clone()).await?),
            verifications: RwLock::new(VerificationsDao::new(database.clone())),
            hosts: RwLock::new(HostsDao::new(database.clone()).await?),
            web_actions: RwLock::new(WebActionsDao::new(database.clone()).await?),
        })
    }
}

/// Bootstrap the database with many migration files, ordered by their filenames within a directory.
/// Reads all SQL files and executes the SQL within. Intended to be used on startup by a database process
/// For testing or any first-time use.
async fn bootstrap<'a>(
    db_con: PooledConnection<'a, PostgresConnectionManager<NoTls>>,
    migration_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    let mut files: Vec<std::fs::DirEntry> = read_dir(migration_dir)
        .expect("reading migration dir failed")
        .map(|r| r.expect("path invalid"))
        .filter(|f| f.file_type().unwrap().is_file())
        .collect();
    files.sort_by_key(|dir| dir.path());

    for file in files {
        debug!(
            "Bootstrapping SQL in {}",
            file.path().canonicalize().unwrap().to_str().unwrap()
        );
        let ddl = read_to_string(file.path()).expect("failed to read SQL file.");
        db_con
            .batch_execute(&ddl)
            .await
            .expect("Failed to execute SQL file.");
    }

    Ok(())
}
