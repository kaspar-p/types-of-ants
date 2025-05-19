use crate::dao::db::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;

pub struct ReleasesDao {
    database: Arc<Mutex<Database>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    #[serde(rename = "releaseNumber")]
    pub release_number: i32,

    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

impl ReleasesDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> ReleasesDao {
        ReleasesDao { database: db }
    }

    pub async fn get_latest_release(&self) -> Result<Release, anyhow::Error> {
        let rows = self
            .database
            .lock()
            .await
            .query(
                "select release_number, created_at from release order by created_at desc limit 1;",
                &[],
            )
            .await?;

        let row: &Row = rows.last().expect("Require at least one release");
        let release_number: i32 = row.get("release_number");
        let created_at: DateTime<Utc> = row.get("created_at");

        return Ok(Release {
            created_at,
            release_number,
        });
    }
}
