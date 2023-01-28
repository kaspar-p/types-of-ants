use postgres::NoTls;
use r2d2::{Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;

pub struct Database {
    pub connection: PooledConnection<PostgresConnectionManager<NoTls>>,
}

pub fn connect(connection_string: String) -> Pool<PostgresConnectionManager<NoTls>> {
    let manager = PostgresConnectionManager::new(connection_string.parse().unwrap(), NoTls);
    let pool = Pool::new(manager).unwrap();
    return pool;
}
