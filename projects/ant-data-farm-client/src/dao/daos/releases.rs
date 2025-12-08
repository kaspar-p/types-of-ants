use crate::{ants::AntId, users::UserId};
use ant_library::db::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};
use tokio::sync::Mutex;
use tokio_postgres::Row;
use uuid::Uuid;

pub struct ReleasesDao {
    database: Arc<Mutex<Database>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub release_number: i32,
    pub release_label: String,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
}

impl Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "[{}:{}]@{}",
            self.release_number, self.release_label, self.created_at,
        ))
    }
}

#[derive(Serialize, Deserialize, Debug)]
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
    let creator_user_id: Uuid = row.get("creator_user_id");

    return Release {
        release_number,
        release_label,
        created_at,
        created_by: UserId(creator_user_id),
    };
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
            .get()
            .await?
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
        let db = self.database.lock().await;
        let mut con = db.get().await?;
        let tx = con.transaction().await?;

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
            (release_number, ant_id, ant_content)
        values
            ($1, $2, $3)
        ",
                &[&release_number, &ant.ant_id.0, &content],
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
            .get()
            .await?
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

#[cfg(test)]
mod test {
    use sha2::{Digest, Sha256};

    fn content_hash(content: &str) -> i32 {
        let hash = &Sha256::digest(content)[..4];
        i32::from_be_bytes(<[u8; 4]>::try_from(hash).expect("failed to convert hash array to i32"))
            .abs()
    }

    #[test]
    fn content_hash_is_positive() {
        let hash = content_hash("ant on the moon");
        println!("{}", hash);
        assert!(hash > 0);
        assert!(content_hash("ant 1") > 0);
        assert!(content_hash("ant 2") > 0);
        assert!(content_hash("ant 3") > 0);
    }
}
