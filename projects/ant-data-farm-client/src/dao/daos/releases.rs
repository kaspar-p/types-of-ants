use crate::{ants::AntId, dao::db::Database, users::UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;

pub struct ReleasesDao {
    database: Arc<Mutex<Database>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Release {
    #[serde(rename = "releaseNumber")]
    pub release_number: i32,

    #[serde(rename = "releaseLabel")]
    pub release_label: String,

    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct AntReleaseRequest {
    #[serde(rename = "antId")]
    pub ant_id: AntId,

    #[serde(rename = "overwriteContent")]
    pub overwrite_content: Option<String>,
}

fn row_to_release(row: &Row) -> Release {
    let release_number: i32 = row.get("release_number");
    let release_label: String = row.get("release_label");
    let created_at: DateTime<Utc> = row.get("created_at");

    return Release {
        release_number,
        release_label,
        created_at,
    };
}

fn content_hash(content: &str) -> i32 {
    let hash = &Sha256::digest(content)[..4];
    i32::from_be_bytes(<[u8; 4]>::try_from(hash).expect("failed to convert hash array to i32"))
}

impl ReleasesDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> ReleasesDao {
        ReleasesDao { database: db }
    }

    pub async fn get_release(&self, release_number: i32) -> Result<Option<Release>, anyhow::Error> {
        let row = self
            .database
            .lock()
            .await
            .query_opt(
                "select
                    release_number, release_label, created_at, creator_user_id
                from release
                    where release_number = $1
                ",
                &[&release_number],
            )
            .await?;

        return Ok(row.map(|r| row_to_release(&r)));
    }

    pub async fn make_release(
        &mut self,
        user_id: &UserId,
        label: String,
        ants: Vec<AntReleaseRequest>,
    ) -> Result<i32, anyhow::Error> {
        let mut db = self.database.lock().await;
        let tx = db.transaction().await?;

        let release_number: i32 = tx
            .query_one(
                "
        insert into release
            (creator_user_id, release_label)
        values
            ($1, $2)
        returning release_number;
        ",
                &[&user_id.0, &label],
            )
            .await?
            .get("release_number");

        for ant in ants {
            let original_content: String = tx
                .query_one(
                    "select suggested_content from ant where ant_id = $1",
                    &[&ant.ant_id.0],
                )
                .await?
                .get("suggested_content");

            let content = ant.overwrite_content.unwrap_or(original_content);
            tx.execute(
                "
        insert into ant_release
            (release_number, ant_id, ant_content, ant_content_hash)
        values
            ($1, $2, $3, $4)
        ",
                &[
                    &release_number,
                    &ant.ant_id.0,
                    &content,
                    &content_hash(&content),
                ],
            )
            .await?;
        }

        tx.commit().await?;

        Ok(release_number)
    }

    pub async fn get_latest_release(&self) -> Result<Release, anyhow::Error> {
        let row = self
            .database
            .lock()
            .await
            .query_one(
                "select
                    release_number, release_label, created_at, creator_user_id
                from release
                order by created_at desc
                limit 1;",
                &[],
            )
            .await?;

        return Ok(row_to_release(&row));
    }
}
