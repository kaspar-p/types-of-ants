mod dao;
mod types;

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use dotenv;
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
    let port = port.unwrap_or(7000);

    if let Err(e) = dotenv::dotenv() {
        panic!("Failed to load environment variables: {}", e);
    }

    let db_name = dotenv::var("DB_PG_NAME")?;
    let user = dotenv::var("DB_PG_USER")?;
    let pw = dotenv::var("DB_PG_PASSWORD")?;

    let connection_string = format!(
        "postgresql://{}:{}@localhost:{}/{}",
        user, pw, port, db_name
    );

    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls).unwrap();
    let pool: Pool<PostgresConnectionManager<NoTls>> =
        Pool::builder().build(manager).await.unwrap();

    Ok(pool)
}

pub async fn connect_port(port: u16) -> Dao {
    let pool = database_connection(Some(port))
        .await
        .unwrap_or_else(|e| panic!("Failed to get environment variable: {}", e));

    debug!("Initializing data access layer...");
    Dao::new(pool).await
}

pub async fn connect() -> Dao {
    let pool = database_connection(None)
        .await
        .unwrap_or_else(|e| panic!("Failed to get environment variable: {e}"));

    debug!("Initializing data access layer...");
    Dao::new(pool).await
}
