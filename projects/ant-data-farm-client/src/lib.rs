mod dao;
mod types;

pub use crate::dao::dao_trait::DaoTrait;
pub use crate::dao::daos::ants;
pub use crate::dao::daos::hosts;
pub use crate::dao::daos::releases;
pub use crate::dao::daos::tweets;
pub use crate::dao::daos::users;
pub use crate::dao::daos::verifications;

use crate::{
    dao::daos::{
        ants::AntsDao, hosts::HostsDao, releases::ReleasesDao, tweets::TweetsDao, users::UsersDao,
        verifications::VerificationsDao,
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
    pub port: Option<u16>,
    /// The credentials to connect to the database.
    /// If omitted, tries to get credentials from ant_library::secret::load_secret and the local fs.
    pub creds: Option<DatabaseCredentials>,
    /// The IP-address host of the database.
    /// If omitted, checks for a $DB_HOST variable, then tries localhost.
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

async fn database_connection(
    config: &DatabaseConfig,
) -> Result<Pool<PostgresConnectionManager<NoTls>>, dotenv::Error> {
    let port = config
        .port
        .unwrap_or(get_port_from_env().unwrap_or(7000))
        .clone();
    let db_creds = config.creds.clone().unwrap_or_else(|| {
        get_credentials_from_env()
            .expect("db: credentials not explicitly passed in must be in the environment!")
    });

    // TODO: find out a more dynamic way of getting the IP of a host
    let host = config
        .host
        .clone()
        .unwrap_or_else(|| get_host_from_env().unwrap_or("localhost".to_owned()));

    let connection_string = format!(
        "postgresql://{}:{}@{}:{}/{}",
        db_creds.database_user, db_creds.database_password, host, port, db_creds.database_name
    );

    debug!(
        "Connecting to database {}:{}/{}",
        host, port, db_creds.database_name
    );
    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        Pool::builder().build(manager).await.unwrap();

    Ok(pool)
}

pub struct AntDataFarmClient {
    pub ants: RwLock<AntsDao>,
    pub releases: RwLock<ReleasesDao>,
    pub users: RwLock<UsersDao>,
    pub verifications: RwLock<VerificationsDao>,
    pub tweets: RwLock<TweetsDao>,
    pub hosts: RwLock<HostsDao>,
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
        let db_con: PooledConnection<'_, PostgresConnectionManager<NoTls>> =
            pool.get_owned().await?;

        let database: Arc<Mutex<Database>> = Arc::new(Mutex::new(db_con));

        let ants = RwLock::new(AntsDao::new(database.clone()).await?);
        let releases = RwLock::new(ReleasesDao::new(database.clone()).await);
        let users = RwLock::new(UsersDao::new(database.clone()).await?);
        let verifications = RwLock::new(VerificationsDao::new(database.clone()));
        let tweets = RwLock::new(TweetsDao::new(database.clone()));
        let hosts = RwLock::new(HostsDao::new(database.clone()).await?);

        Ok(AntDataFarmClient {
            ants,
            releases,
            tweets,
            users,
            verifications,
            hosts,
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
