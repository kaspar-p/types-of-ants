mod dao;
mod types;

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;

use tokio_postgres::NoTls;
use tracing::debug;

pub use crate::dao::dao::Dao;
pub use crate::dao::dao_trait::DaoTrait;
pub use crate::dao::daos::ants;
pub use crate::dao::daos::hosts;
pub use crate::dao::daos::users;

#[derive(Debug, Clone)]
pub struct DatabaseCredentials {
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
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

async fn internal_connect(config: DatabaseConfig) -> Result<Dao, anyhow::Error> {
    let pool = database_connection(config)
        .await
        .unwrap_or_else(|e| panic!("Failed to get environment variable: {e}"));

    debug!("Initializing data access layer...");
    let dao = Dao::new(pool).await;
    debug!("Data access layer initialized!");
    return dao;
}

pub async fn connect() -> Result<Dao, anyhow::Error> {
    internal_connect(DatabaseConfig {
        port: None,
        creds: None,
        host: None,
    })
    .await
}

pub async fn connect_config(config: DatabaseConfig) -> Result<Dao, anyhow::Error> {
    internal_connect(config).await
}
