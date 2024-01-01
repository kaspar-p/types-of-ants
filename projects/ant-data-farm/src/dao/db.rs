pub type Database =
    bb8::PooledConnection<'static, bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;
