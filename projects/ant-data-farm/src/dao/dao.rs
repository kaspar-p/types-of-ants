use super::{
    dao_trait::DaoTrait,
    daos::{ants::AntsDao, hosts::HostsDao, releases::ReleasesDao, users::UsersDao},
};
use crate::{dao::db::Database, types::ConnectionPool};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
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
    /// If omitted, tries to get credentials from $DB_PG_NAME, DB_PG_PASSWORD,
    /// and DB_PG_USER environment variables
    pub creds: Option<DatabaseCredentials>,
    /// The IP-address host of the database.
    /// If omitted, checks for a $DB_HOST variable, then tries localhost.
    pub host: Option<String>,
}

fn get_credentials_from_env() -> Result<DatabaseCredentials, dotenv::Error> {
    dotenv::dotenv()?;
    Ok(DatabaseCredentials {
        database_name: dotenv::var("DB_PG_NAME")?,
        database_user: dotenv::var("DB_PG_USER")?,
        database_password: dotenv::var("DB_PG_PASSWORD")?,
    })
}

fn get_port_from_env() -> Option<u16> {
    dotenv::dotenv().ok()?;
    dotenv::var("DB_PG_PORT").ok()?.parse::<u16>().ok()
}

fn get_host_from_env() -> Option<String> {
    dotenv::dotenv().ok()?;
    dotenv::var("DB_HOST").ok()
}

async fn database_connection(
    config: DatabaseConfig,
) -> Result<Pool<PostgresConnectionManager<NoTls>>, dotenv::Error> {
    let port = config
        .port
        .unwrap_or(get_port_from_env().unwrap_or(7000))
        .clone();
    let db_creds = config.creds.unwrap_or_else(|| {
        get_credentials_from_env()
            .expect("Credentials not explicitly passed in must be in the environment!")
    });

    // TODO: find out a more dynamic way of getting the IP of a host
    let host = config
        .host
        .unwrap_or_else(|| get_host_from_env().unwrap_or("localhost".to_owned()));

    let connection_string = format!(
        "postgresql://{}:{}@{}:{}/{}",
        db_creds.database_user, db_creds.database_password, host, port, db_creds.database_name
    );

    debug!("Connecting to database at port {port}...");
    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        Pool::builder().build(manager).await.unwrap();

    Ok(pool)
}

pub struct AntDataFarmClient {
    pub ants: RwLock<AntsDao>,
    pub releases: RwLock<ReleasesDao>,
    pub users: RwLock<UsersDao>,
    pub hosts: RwLock<HostsDao>,
    // pub deployments: DeploymentsDao,
    // pub metrics: MetricsDao,
    // pub tests: TestsDao,
}

impl AntDataFarmClient {
    pub async fn new(config: DatabaseConfig) -> Result<AntDataFarmClient, anyhow::Error> {
        let pool = database_connection(config).await?;
        info!("Initializing data access layer...");
        let client = AntDataFarmClient::initialize(pool).await?;
        return Ok(client);
    }

    async fn initialize(pool: ConnectionPool) -> Result<AntDataFarmClient, anyhow::Error> {
        let db_con = pool.get_owned().await?;

        let database: Arc<Mutex<Database>> = Arc::new(Mutex::new(db_con));

        let ants = RwLock::new(AntsDao::new(database.clone()).await?);
        let releases = RwLock::new(ReleasesDao::new(database.clone()).await?);
        let hosts = RwLock::new(HostsDao::new(database.clone()).await?);
        let users = RwLock::new(UsersDao::new(database.clone()).await?);

        Ok(AntDataFarmClient {
            ants,
            releases,
            users,
            hosts,
        })
    }
}
