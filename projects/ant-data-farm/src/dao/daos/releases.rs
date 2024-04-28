use crate::dao::db::Database;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ReleasesDao {
    database: Arc<Mutex<Database>>,
    latest_release: i32,
}

impl ReleasesDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> Result<ReleasesDao, anyhow::Error> {
        let rows = db
            .lock()
            .await
            .query(
                "select max(release_number) as latest_release from release limit 1",
                &[],
            )
            .await?;
        let latest_release: i32 = rows.last().expect("had last").get("latest_release");

        Ok(ReleasesDao {
            latest_release,
            database: db,
        })
    }

    pub async fn get_latest_release(&self) -> i32 {
        self.latest_release
    }
}
