pub use super::lib::Id as AntId;
use super::users::UserId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};
use tokio::sync::Mutex;
use tokio_postgres::Row;

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
    #[serde(rename = "unreleased")]
    Unreleased,

    #[serde(rename = "released")]
    Released(i32),

    #[serde(rename = "declined")]
    Declined,
}

impl Display for AntStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AntStatus::Declined => f.write_str("declined"),
            AntStatus::Released(release_number) => {
                f.write_fmt(format_args!("released::{}", release_number))
            }
            AntStatus::Unreleased => f.write_str("unreleased"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd)]
pub enum Tweeted {
    #[serde(rename = "notTweeted")]
    NotTweeted,

    #[serde(rename = "tweeted")]
    Tweeted(DateTime<Utc>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ant {
    #[serde(rename = "antId")]
    pub ant_id: AntId,

    #[serde(rename = "antName")]
    pub ant_name: String,

    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[serde(rename = "createdBy")]
    pub created_by: UserId,

    #[serde(rename = "createdByUsername")]
    pub created_by_username: String,

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
}

#[async_trait::async_trait]
impl DaoTrait<AntsDao, Ant> for AntsDao {
    async fn new(db: Arc<Mutex<Database>>) -> Result<AntsDao, anyhow::Error> {
        Ok(AntsDao { database: db })
    }

    // Read
    async fn get_all(&self) -> anyhow::Result<Vec<Ant>> {
        let rows = self
            .database
            .lock()
            .await
            .get()
            .await?
            .query(
                "
            select 
                ant.ant_id, 
                ant.suggested_content,
                ant_release.ant_content, 
                ant_release.release_number, 
                ant.created_at,
                registered_user.user_name,
                ant.ant_user_id,
                ant_declined.ant_declined_at,
                ant_tweeted.tweeted_at
            from 
                ant left join ant_release on ant.ant_id = ant_release.ant_id
                    left join ant_declined on ant.ant_id = ant_declined.ant_id
                    left join ant_tweeted on ant.ant_id = ant_tweeted.ant_id
                    left join registered_user on ant.ant_user_id = registered_user.user_id
            order by ant_release.ant_content_hash
            ",
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|row| row_to_ant(&row)).collect())
    }

    async fn get_one_by_id(&self, ant_id: &AntId) -> Result<Option<Ant>> {
        let rows = self
            .database
            .lock()
            .await
            .get()
            .await?
            .query(
                "
            select 
                ant.ant_id, 
                ant.suggested_content,
                ant_release.ant_content, 
                ant_release.release_number, 
                ant.created_at,
                registered_user.user_name,
                ant.ant_user_id,
                ant_declined.ant_declined_at,
                ant_tweeted.tweeted_at
            from 
                ant left join ant_release on ant.ant_id = ant_release.ant_id
                    left join ant_declined on ant.ant_id = ant_declined.ant_id
                    left join ant_tweeted on ant.ant_id = ant_tweeted.ant_id
                    left join registered_user on ant.ant_user_id = registered_user.user_id
            where
                ant.ant_id = $1
            ",
                &[&ant_id.0],
            )
            .await?;

        return Ok(rows.first().map(|row| row_to_ant(row)));
    }
}

fn row_to_ant(row: &Row) -> Ant {
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
        created_by_username: row.get("user_name"),
        created_by: row.get("ant_user_id"),
        status,
        tweeted: tweeted_status,
    }
}

impl AntsDao {
    pub async fn get_user_feed_since(
        &self,
        user_id: &UserId,
        since: &DateTime<Utc>,
    ) -> Result<Option<Vec<Ant>>, anyhow::Error> {
        return Ok(Some(vec![]));
    }

    pub async fn is_favorite_ant(
        &self,
        user: &UserId,
        ant: &AntId,
    ) -> Result<Option<DateTime<Utc>>> {
        let db = self.database.lock().await;
        let con = db.get().await?;

        let favorite_row = con
            .query_opt(
                "select user_id, ant_id, favorited_at
 from favorite
 where
    user_id = $1 and
    ant_id = $2
limit 1",
                &[&user.0, &ant.0],
            )
            .await?;

        return Ok(favorite_row.map(|r| r.get("favorited_at")));
    }

    pub async fn favorite_ant(&mut self, user: &UserId, ant: &AntId) -> Result<DateTime<Utc>> {
        let db = self.database.lock().await;
        let mut con = db.get().await?;
        let tx = con.transaction().await?;

        let favorited_at: DateTime<Utc> = tx
            .query_one(
                "
        insert into favorite
            (user_id, ant_id)
        values
            ($1, $2)
        returning favorited_at
        ",
                &[&user.0, &ant.0],
            )
            .await?
            .get("favorited_at");

        tx.commit().await?;

        Ok(favorited_at)
    }

    pub async fn unfavorite_ant(&mut self, user: &UserId, ant: &AntId) -> Result<()> {
        let db = self.database.lock().await;
        let mut con = db.get().await?;
        let tx = con.transaction().await?;

        let rows = tx
            .execute(
                "
            delete from favorite
            where
                user_id = $1 and
                ant_id = $2
            ",
                &[&user.0, &ant.0],
            )
            .await?;

        if rows != 1 {
            return Err(anyhow::Error::msg(format!(
                "Unexpectedly changed {rows} rows"
            )));
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn is_ant_declined(&self, ant: &AntId) -> Result<bool> {
        let ant_row = self
            .database
            .lock()
            .await
            .get()
            .await?
            .query_opt(
                "
            select (ant_id, ant_declined_user_id)
            from ant_declined
            where ant_id = $1",
                &[&ant.0],
            )
            .await?;

        Ok(ant_row.is_some())
    }

    // Assumes the ant is not already declined!
    pub async fn decline_ant(&mut self, user: &UserId, ant: &AntId) -> Result<DateTime<Utc>> {
        let declined_at: DateTime<Utc> = self
            .database
            .lock()
            .await
            .get()
            .await?
            .query_one(
                "
    insert into ant_declined
        (ant_declined_user_id, ant_id)
    values
        ($1, $2)
    returning ant_declined_at",
                &[&user.0, &ant.0],
            )
            .await?
            .get("ant_declined_at");

        Ok(declined_at)
    }

    pub async fn add_ant_tweet(&mut self, ant: &AntId) -> Result<Ant> {
        let time = chrono::offset::Utc::now();

        let _ = self
            .database
            .lock()
            .await
            .get()
            .await?
            .execute(
                "
            insert into ant_tweeted
                (ant_id, tweeted_at)
            values
                ($1::uuid, $2::timestamptz)
            limit 1",
                &[&ant.0, &time],
            )
            .await?;

        let mut ant = self
            .get_one_by_id(ant)
            .await?
            .ok_or(anyhow::Error::msg("No such ant."))?;
        ant.tweeted = Tweeted::Tweeted(time);
        Ok(ant)
    }

    pub async fn get_all_released(&self) -> Result<Vec<Ant>> {
        Ok(self
            .get_all()
            .await?
            .into_iter()
            .filter(|ant| match ant.status {
                AntStatus::Released(_) => true,
                _ => false,
            })
            .collect::<Vec<Ant>>())
    }

    pub async fn add_unreleased_ant(
        &mut self,
        ant_suggestion_content: String,
        user_id: UserId,
        username: String,
    ) -> Result<Ant, anyhow::Error> {
        let ant = Ant {
            ant_id: AntId(uuid::Uuid::new_v4()),
            ant_name: ant_suggestion_content,
            created_at: chrono::offset::Utc::now(),
            created_by: user_id,
            created_by_username: username,
            tweeted: Tweeted::NotTweeted,
            status: AntStatus::Unreleased,
        };

        self.database
            .lock()
            .await
            .get()
            .await?
            .execute(
                "
            insert into ant
                (ant_id, suggested_content, ant_user_id)
            values
                ($1::uuid, $2, $3::uuid)
            limit 1",
                &[&ant.ant_id.0, &ant.ant_name, &ant.created_by.0],
            )
            .await?;

        Ok(ant)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ant_tweeted() {
        assert_eq!(1, 1)
    }
}
