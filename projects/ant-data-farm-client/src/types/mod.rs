use bb8::Pool;
use bb8_postgres::{tokio_postgres::NoTls, PostgresConnectionManager};

pub type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;
