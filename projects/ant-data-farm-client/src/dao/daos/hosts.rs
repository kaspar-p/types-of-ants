pub use super::lib::Id as HostId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Host {
    pub host_id: HostId,
    pub host_label: String,
    pub host_location: String,
    pub host_hostname: String,
    pub host_type: String,
    pub host_os: String,
}

pub struct HostsDao {
    database: Arc<Mutex<Database>>,
}

fn row_to_host(row: &Row) -> Host {
    Host {
        host_id: row.get("host_id"),
        host_label: row.get("host_label"),
        host_location: row.get("host_location"),
        host_hostname: row.get("host_hostname"),
        host_type: row.get("host_type"),
        host_os: row.get("host_os"),
    }
}

impl HostsDao {
    pub async fn get_one_by_hostname(&self, hostname: &str) -> Result<Option<Host>, anyhow::Error> {
        Ok(self
            .database
            .lock()
            .await
            .get()
            .await?
            .query(
                "
                        select host_id, host_label, host_location, host_hostname, host_type, host_os
                        from host
                        where host_hostname = $1 or host_label = $1
                        limit 1",
                &[&hostname],
            )
            .await?
            .first()
            .map(|row| row_to_host(row)))
    }
}

#[async_trait]
impl DaoTrait<HostsDao, Host> for HostsDao {
    async fn new(db: Arc<Mutex<Database>>) -> Result<HostsDao, anyhow::Error> {
        Ok(HostsDao { database: db })
    }

    async fn get_all(&self) -> Result<Vec<Host>> {
        Ok(self.database
                .lock()
                .await.get().await?
                .query("select host_id, host_label, host_location, host_hostname, host_type, host_os from host;", &[])
                .await?
                .iter()
                .map(|row| row_to_host(row))
                .collect())
    }

    async fn get_one_by_id(&self, host_id: &HostId) -> Result<Option<Host>> {
        Ok(self
            .database
            .lock()
            .await
            .get()
            .await?
            .query(
                "
                select host_id, host_label, host_location, host_hostname, host_type, host_os
                from host
                where host_id = $1
                limit 1;",
                &[&host_id.0],
            )
            .await?
            .first()
            .map(|row| row_to_host(row)))
    }
}
