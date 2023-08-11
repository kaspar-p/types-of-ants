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

async fn database_connection(
    port: Option<u16>,
) -> Result<Pool<PostgresConnectionManager<NoTls>>, dotenv::Error> {
    if let Err(e) = dotenv::dotenv() {
        panic!("Failed to load environment variables: {e}");
    }

    let env_port = dotenv::var("DB_PG_PORT")?;
    let port = port.unwrap_or(env_port.parse::<u16>().unwrap());

    let db_name = dotenv::var("DB_PG_NAME")?;
    let user = dotenv::var("DB_PG_USER")?;
    let pw = dotenv::var("DB_PG_PASSWORD")?;

    let connection_string = format!(
        "postgresql://{}:{}@localhost:{}/{}",
        user, pw, port, db_name
    );

    debug!("Connecting to database at port {port}...");
    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        Pool::builder().build(manager).await.unwrap();

    Ok(pool)
}

async fn internal_connect(port: Option<u16>) -> Dao {
    let pool = database_connection(port)
        .await
        .unwrap_or_else(|e| panic!("Failed to get environment variable: {e}"));

    debug!("Initializing data access layer...");
    let dao = Dao::new(pool).await;
    debug!("Data access layer initialized!");
    return dao;
}

pub async fn connect_port(port: u16) -> Dao {
    internal_connect(Some(port)).await
}

pub async fn connect() -> Dao {
    internal_connect(None).await
}
