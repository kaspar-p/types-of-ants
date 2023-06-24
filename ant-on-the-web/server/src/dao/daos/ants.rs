pub use super::lib::Id as AntId;
use super::users::UserId;
use crate::dao::db::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum SuggestionStatus {
//     LIVE,
//     DECLINED,
//     DEPRECATED,
//     UNSEEN,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SuggestionAudit {
//     pub who: UserId,
//     pub when: DateTime<Utc>,
//     pub status: SuggestionStatus,
// }

#[derive(Clone, Serialize, Deserialize)]
pub enum AntStatus {
    UNRELEASED,
    RELEASED(i32),
    DECLINED,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ant {
    pub ant_id: AntId,
    pub ant_name: String,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub status: AntStatus,
}

async fn construct_ant_status(db: Arc<Mutex<Database>>, ant_id: AntId) -> AntStatus {
    let rows = db
        .lock()
        .await
        .query(
            "select release_number from ant_release where ant_id = $1",
            &[&ant_id],
        )
        .await
        .unwrap_or_else(|_| panic!("Failed to get releases for ant ID {}", ant_id));

    if rows.is_empty() || rows[0].is_empty() {
        return AntStatus::UNRELEASED;
    } else {
        return AntStatus::RELEASED(rows[0].get("release_number"));
    }
}

pub struct AntsDao {
    database: Arc<Mutex<Database>>,
    current_release: i32,
    ants_by_id: HashMap<AntId, Ant>,
    ants_by_name: HashMap<String, Ant>,
}

impl AntsDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> AntsDao {
        let mut ants_by_id = HashMap::<AntId, Ant>::new();
        let mut ants_by_name = HashMap::<String, Ant>::new();

        let rows = db
            .lock()
            .await
            .query(
                "select ant_id, ant_user_id, created_at, ant_content from ant",
                &[],
            )
            .await
            .unwrap_or_else(|_| panic!("Getting ant data failed!"));

        let all_ants = futures::future::join_all(rows.iter().map(|row| async {
            Ant {
                ant_id: row.get("ant_id"),
                ant_name: row.get("ant_content"),
                created_at: row.get("created_at"),
                created_by: row.get("ant_user_id"),
                status: construct_ant_status(db.clone(), row.get("ant_id")).await,
            }
        }))
        .await;

        for ant in all_ants {
            ants_by_id.insert(ant.ant_id.clone(), ant.clone());
            ants_by_name.insert(ant.ant_name.clone(), ant.clone());
        }

        let row = db
            .lock()
            .await
            .query(
                "select max(release_number) as current_release from release;",
                &[],
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to get release number from DB!"))
            .pop();
        if row.is_none() {
            panic!("No release number found in the DB!");
        }

        let current_release: i32 = match row.unwrap().try_get("current_release") {
            Err(e) => panic!("No release number found in the DB: {}", e),
            Ok(n) => n,
        };

        AntsDao {
            database: db,
            current_release,
            ants_by_id,
            ants_by_name,
        }
    }

    pub fn get_current_release(&self) -> i32 {
        self.current_release
    }

    pub fn get_ant_by_id(&self, ant_id: AntId) -> Option<Ant> {
        self.get_ant_by_id(ant_id).clone()
    }

    pub fn get_ant_by_name(&self, ant_name: String) -> Option<Ant> {
        self.get_ant_by_name(ant_name).clone()
    }

    pub fn get_all_ants(&self) -> Vec<Ant> {
        self.ants_by_id
            .values()
            .into_iter()
            .map(|x| x.clone())
            .collect::<Vec<Ant>>()
    }
}
