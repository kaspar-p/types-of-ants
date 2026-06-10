use std::{fs::read_to_string, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use std::fs::read_dir;
use tracing::debug;

use crate::sd::pg::PostgresManager;
use crate::sd::reader::ServiceDiscovery;

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
    /// Executes in the order of the vector.
    pub migration_dirs: Vec<PathBuf>,
}

/// Credentials and migration config without a static host/port — used with service discovery.
#[derive(Debug, Clone)]
pub struct DatabaseCredentialsConfig {
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
    pub migration_dirs: Vec<PathBuf>,
}

/// A bb8 connection pool backed by `PostgresManager`. For static connections the
/// manager holds a fixed endpoint; for SD-based connections it re-resolves on every checkout
/// and recycles connections when the endpoint changes.
pub type ConnectionPool = Pool<PostgresManager>;

/// Build a static connection pool from a `DatabaseConfig` (host/port known at startup).
pub async fn database_connection(config: &DatabaseConfig) -> Result<ConnectionPool, anyhow::Error> {
    debug!(
        "Connecting to database postgresql://{}:{}/{}",
        config.host, config.port, config.database_name
    );

    let manager = PostgresManager::new_static(
        &config.host,
        config.port,
        &config.database_name,
        &config.database_user,
        &config.database_password,
    );
    let db: ConnectionPool = Pool::builder().build(manager).await?;

    for migration_dir in &config.migration_dirs {
        let con = db.get().await?;
        bootstrap(con, migration_dir).await?;
    }

    Ok(db)
}

/// Build a dynamic connection pool whose host/port are resolved via Consul on every
/// new connection. Connections are automatically recycled when the endpoint changes.
pub async fn database_connection_dynamic(
    sd: Arc<ServiceDiscovery>,
    service: impl Into<String>,
    config: &DatabaseCredentialsConfig,
) -> Result<ConnectionPool, anyhow::Error> {
    let service = service.into();
    debug!("Connecting to database '{}' via service discovery", service);

    let manager = PostgresManager::new_dynamic(
        sd,
        service,
        &config.database_name,
        &config.database_user,
        &config.database_password,
    );
    let db: ConnectionPool = Pool::builder().build(manager).await?;

    for migration_dir in &config.migration_dirs {
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
async fn bootstrap<'a>(
    db_con: PooledConnection<'a, PostgresManager>,
    migration_dir: &PathBuf,
) -> Result<(), anyhow::Error> {
    let mut files: Vec<std::fs::DirEntry> = read_dir(migration_dir)
        .expect("reading migration dir failed")
        .map(|r| r.expect("path invalid"))
        .filter(|f| f.file_type().unwrap().is_file())
        .collect();
    files.sort_by_key(|dir| dir.path());

    for file in files {
        debug!("Bootstrapping with: {}", file.path().display());

        match file
            .path()
            .extension()
            .expect("file had no extension")
            .to_str()
            .unwrap()
        {
            "sql" => {
                let ddl = read_to_string(file.path()).expect("failed to read SQL file.");
                db_con
                    .batch_execute(&ddl)
                    .await
                    .expect("Failed to execute SQL file.");
            }
            "sh" => {
                std::process::Command::new(file.path())
                    .output()
                    .expect("Failed to execute bootstrap shell file.");
            }
            _ => continue,
        }
    }

    Ok(())
}
