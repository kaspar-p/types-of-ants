use bb8_postgres::{bb8::Pool, PostgresConnectionManager};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, Row};
use tracing::debug;

#[derive(Clone)]
pub struct AntBackingItUpStorageClient {
    db: Pool<PostgresConnectionManager<NoTls>>,
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
        let connection_string = AntBackingItUpStorageClient::make_connection_string(
            &params.username,
            &params.password,
            &params.host,
            params.port,
            &params.db_name,
        );

        debug!(
            "Connecting to database {}",
            AntBackingItUpStorageClient::make_connection_string(
                "[redacted]",
                "[redacted]",
                &params.host,
                params.port,
                &params.db_name
            )
        );
        let pool_manager = PostgresConnectionManager::new_from_stringlike(connection_string, NoTls)
            .expect("db connection");
        let pool = Pool::builder()
            .build(pool_manager)
            .await
            .expect("db connection failed");

        Ok(Self { db: pool })
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
        destination_host: &str,
        destination_port: u16,
        destination_filepath: &str,
    ) -> Result<DateTime<Utc>, anyhow::Error> {
        let created_at: DateTime<Utc> = self
            .db
            .get()
            .await?
            .query_one(
                "
                insert into backup
                  (project, database_host, database_port, encryption_nonce, destination_host, destination_port, destination_filepath)
                values
                  ($1, $2, $3, $4, $5, $6, $7)
                returning created_at
              ",
                &[&project, &source.host, &(source.port as i32), encryption_nonce, &destination_host, &(destination_port as i32), &destination_filepath],
            )
            .await?
            .get("created_at");

        Ok(created_at)
    }
}
