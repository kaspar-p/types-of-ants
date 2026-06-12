use ant_library::sd::pg::{make_connection_string, PostgresManager};
use ant_library::sd::reader::ServiceDiscovery;
use bb8_postgres::bb8::Pool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::Row;
use tracing::debug;

#[derive(Clone)]
pub struct AntBackingItUpStorageClient {
    db: Pool<PostgresManager>,
}

pub struct DatabaseParams {
    pub host: String,
    pub port: u16,
    pub db_name: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub backup_id: String,
    pub project: String,
    pub database_host: String,
    pub database_port: u16,
    pub encryption_nonce: Vec<u8>,
    pub destination_host: String,
    pub destination_port: u16,
    pub destination_filepath: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn row_to_backup(row: &Row) -> Backup {
    Backup {
        backup_id: row.get("backup_id"),
        project: row.get("project"),
        database_host: row.get("database_host"),
        database_port: row.get::<_, i32>("database_port") as u16,
        encryption_nonce: row.get("encryption_nonce"),
        destination_host: row.get("destination_host"),
        destination_port: row.get::<_, i32>("destination_port") as u16,
        destination_filepath: row.get("destination_filepath"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

impl AntBackingItUpStorageClient {
    pub async fn connect(params: &DatabaseParams) -> Result<Self, anyhow::Error> {
        debug!(
            "Connecting to database {}",
            make_connection_string(
                "[redacted]",
                "[redacted]",
                &params.host,
                params.port,
                &params.db_name
            )
        );

        let manager = PostgresManager::new_static(
            &params.host,
            params.port,
            &params.db_name,
            &params.username,
            &params.password,
        );
        let pool = Pool::builder().build(manager).await?;
        Ok(Self { db: pool })
    }

    /// Connect via Consul service discovery. The pool re-resolves "ant-backing-it-up-db" on
    /// every new connection and recycles connections when the endpoint changes.
    pub async fn connect_discovered(sd: &ServiceDiscovery) -> Result<Self, anyhow::Error> {
        let manager = PostgresManager::new_dynamic(
            Arc::new(sd.clone()),
            "ant-backing-it-up-db",
            ant_library::secret::load_secret("ant_backing_it_up_db_db")?,
            ant_library::secret::load_secret("ant_backing_it_up_db_user")?,
            ant_library::secret::load_secret("ant_backing_it_up_db_password")?,
        );
        let pool = Pool::builder().build(manager).await?;
        Ok(Self { db: pool })
    }

    pub async fn get_latest_backup_for_project(
        &self,
        project: &str,
    ) -> Result<Option<Backup>, anyhow::Error> {
        let row = self
            .db
            .get()
            .await?
            .query_opt(
                "
      select
        backup_id,
        project,
        database_host,
        database_port,
        encryption_nonce,
        destination_host,
        destination_port,
        destination_filepath,
        created_at,
        updated_at
      from backup
      where project = $1
      order by created_at desc
      limit 1",
                &[&project],
            )
            .await?;

        Ok(row.map(|row| row_to_backup(&row)))
    }

    pub async fn get_all_backups(&self) -> Result<Vec<Backup>, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .query(
                "
      select
        backup_id,
        project,
        database_host,
        database_port,
        encryption_nonce,
        destination_host,
        destination_port,
        destination_filepath,
        created_at,
        updated_at
      from backup
      ",
                &[],
            )
            .await?
            .into_iter()
            .map(|row| row_to_backup(&row))
            .collect();

        Ok(rows)
    }

    pub async fn record_backup(
        &mut self,
        project: &str,
        source: &DatabaseParams,
        encryption_nonce: &Vec<u8>,
        destination_filepath: &str,
    ) -> Result<DateTime<Utc>, anyhow::Error> {
        let created_at: DateTime<Utc> = self
            .db
            .get()
            .await?
            .query_one(
                "
                insert into backup
                  (project, database_host, database_port, encryption_nonce, destination_filepath)
                values
                  ($1, $2, $3, $4, $5, $6, $7)
                returning created_at
              ",
                &[
                    &project,
                    &source.host,
                    &(source.port as i32),
                    encryption_nonce,
                    &destination_filepath,
                ],
            )
            .await?
            .get("created_at");

        Ok(created_at)
    }
}
