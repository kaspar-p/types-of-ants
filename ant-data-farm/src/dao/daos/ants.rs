pub use super::lib::Id as AntId;
use super::users::UserId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use chrono::{DateTime, Utc};
use double_map::DHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AntStatus {
    Unreleased,
    Released(i32),
    Declined,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd)]
pub enum Tweeted {
    NotTweeted,
    Tweeted(DateTime<Utc>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ant {
    pub ant_id: AntId,
    pub ant_name: String,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub tweeted: Tweeted,
    pub status: AntStatus,
}

impl PartialOrd for Ant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return self.created_at.partial_cmp(&other.created_at);
    }
}

impl Ord for Ant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return self.created_at.cmp(&other.created_at);
    }
}

pub struct AntsDao {
    database: Arc<Mutex<Database>>,
    ants: DHashMap<AntId, String, Box<Ant>>,
}

#[async_trait::async_trait]
impl DaoTrait<Ant> for AntsDao {
    async fn new(db: Arc<Mutex<Database>>) -> AntsDao {
        let mut ants = DHashMap::<AntId, String, Box<Ant>>::new();

        let released_ant_rows = db
            .lock()
            .await
            .query(
                "select 
                    ant.ant_id, 
                    ant.suggested_content,
                    ant_release.ant_content, 
                    ant_release.release_number, 
                    ant.created_at,
                    ant.ant_user_id,
                    ant_declined.ant_declined_at,
                    ant_tweeted.tweeted_at
                from 
                    ant left join ant_release on ant.ant_id = ant_release.ant_id
                        left join ant_declined on ant.ant_id = ant_declined.ant_id
                        left join ant_tweeted on ant.ant_id = ant_tweeted.ant_id
                ",
                &[],
            )
            .await
            .unwrap_or_else(|e| panic!("Getting ant data failed: {e}"));

        let released_ants = futures::future::join_all(released_ant_rows.iter().map(|row| async {
            let tweeted_at: Option<DateTime<Utc>> = row.get("tweeted_at");
            let ant_content: Option<String> = row.get("ant_content");
            let declined_at: Option<DateTime<Utc>> = row.get("ant_declined_at");

            let status = {
                if ant_content.is_some() {
                    AntStatus::Released(row.get("release_number"))
                } else if declined_at.is_some() {
                    AntStatus::Declined
                } else {
                    AntStatus::Unreleased
                }
            };

            let tweeted_status = if tweeted_at.is_some() {
                Tweeted::Tweeted(row.get("tweeted_at"))
            } else {
                Tweeted::NotTweeted
            };

            let content: String = if let Some(name) = row.get("ant_content") {
                name
            } else {
                row.get("suggested_content")
            };

            Ant {
                ant_id: row.get("ant_id"),
                ant_name: content,
                created_at: row.get("created_at"),
                created_by: row.get("ant_user_id"),
                status,
                tweeted: tweeted_status,
            }
        }))
        .await;

        for ant in released_ants {
            ants.insert(ant.ant_id, ant.ant_name.clone(), Box::new(ant));
        }

        AntsDao { database: db, ants }
    }

    // Read
    async fn get_all(&self) -> Vec<&Ant> {
        self.ants
            .values()
            .into_iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<&Ant>>()
    }

    async fn get_all_mut(&mut self) -> Vec<&mut Ant> {
        vec![]
    }

    async fn get_one_by_id(&self, ant_id: &AntId) -> Option<&Ant> {
        Some(self.ants.get_key1(ant_id)?.as_ref())
    }

    async fn get_one_by_id_mut(&mut self, ant_id: &AntId) -> Option<&mut Ant> {
        let ant = self.ants.get_mut_key1(ant_id)?;
        Some(ant.as_mut())
    }

    async fn get_one_by_name(&self, ant_name: &str) -> Option<&Ant> {
        Some(self.ants.get_key2(ant_name)?.as_ref())
    }

    async fn get_one_by_name_mut(&mut self, ant_name: &str) -> Option<&mut Ant> {
        Some(self.ants.get_mut_key2(ant_name)?.as_mut())
    }
}

impl AntsDao {
    pub async fn add_ant_tweet(&mut self, ant: &AntId) -> Option<&Ant> {
        let time = chrono::offset::Utc::now();

        let res_affected = self
            .database
            .lock()
            .await
            .execute(
                "insert into ant_tweeted (ant_id, tweeted_at) values ($1::uuid, $2::timestamptz) limit 1",
                &[&ant.0, &time],
            )
            .await;
        if res_affected.is_err() {
            debug!(
                "Error inserting ant tweet with ID '{}': {}",
                ant,
                res_affected.unwrap_err()
            );
            return None;
        }

        let ant = self.get_one_by_id_mut(ant).await?;
        ant.tweeted = Tweeted::Tweeted(time);
        Some(ant)
    }

    pub async fn add_unreleased_ant(
        &mut self,
        ant_suggestion_content: String,
        user_id: UserId,
    ) -> Result<(), tokio_postgres::Error> {
        let ant = Ant {
            ant_id: AntId(uuid::Uuid::new_v4()),
            ant_name: ant_suggestion_content,
            created_at: chrono::offset::Utc::now(),
            created_by: user_id,
            tweeted: Tweeted::NotTweeted,
            status: AntStatus::Unreleased,
        };

        let changed = self
            .database
            .lock()
            .await
            .execute(
                "insert into ant (ant_id, suggested_content, ant_user_id) values ($1::uuid, $2, $3::uuid) limit 1",
                &[&ant.ant_id.0, &ant.ant_name, &ant.created_by.0],
            )
            .await;
        if let Err(e) = changed {
            return Result::Err(e);
        }

        self.ants.insert(
            ant.ant_id.clone(),
            ant.ant_name.clone(),
            Box::new(ant.clone()),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ant_tweeted() {
        assert_eq!(1, 1)
    }
}
