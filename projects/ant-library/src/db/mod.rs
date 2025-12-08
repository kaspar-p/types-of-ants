use std::{fs::read_to_string, path::PathBuf};

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use std::fs::read_dir;
use tokio_postgres::NoTls;
use tracing::debug;

pub mod fixture;

#[derive(Debug, Clone)]
pub struct DatabaseCredentials {}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// The database port.
    pub port: u16,
    /// The credentials to connect to the database.
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
    /// The host of the database.
    pub host: String,
    /// On client startup, execute the SQL within the directory to bootstrap schemas and databases.
    pub migration_dir: Option<PathBuf>,
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

pub type Database = Pool<PostgresConnectionManager<NoTls>>;

pub async fn database_connection(config: &DatabaseConfig) -> Result<Database, anyhow::Error> {
    let connection_string = make_connection_string(
        &config.database_user,
        &config.database_password,
        &config.host,
        config.port,
        &config.database_name,
    );

    debug!(
        "Connecting to database {}",
        make_connection_string(
            "[redacted]",
            "[redacted]",
            &config.host,
            config.port,
            &config.database_name
        )
    );
    let manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls)?;
    let db: Database = Pool::builder().build(manager).await?;

    if let Some(migration_dir) = &config.migration_dir {
        let con = db.get().await?;
        bootstrap(con, migration_dir).await?;
    }

    Ok(db)
}

#[async_trait]
pub trait TypesOfAntsDatabase: Sized {
    async fn connect(config: &DatabaseConfig) -> Result<Self, anyhow::Error>;
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
