use std::sync::Arc;

use super::lib::Id;
use crate::ants::AntId;
use crate::dao::db::Database;
use crate::users::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_postgres::Row;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledTweet {
    pub scheduled_tweet_id: Id,
    pub scheduled_at: DateTime<Utc>,
    pub scheduled_by_user_name: String,
    pub tweet_prefix: Option<String>,
    pub tweet_suffix: Option<String>,
    pub ants_to_tweet: Vec<TweetAnt>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TweetAnt {
    pub ant_id: AntId,
    pub ant_content: String,
}

pub struct TweetsDao {
    database: Arc<Mutex<Database>>,
}

fn row_to_scheduled_tweet(row: &Row, user_name: String, ants: Vec<TweetAnt>) -> ScheduledTweet {
    return ScheduledTweet {
        scheduled_tweet_id: row.get("scheduled_tweet_id"),
        scheduled_at: row.get("scheduled_at"),
        scheduled_by_user_name: user_name,
        tweet_prefix: row.get("tweet_prefix"),
        tweet_suffix: row.get("tweet_suffix"),
        ants_to_tweet: ants,
    };
}

fn row_to_tweet_ant(row: Row) -> TweetAnt {
    TweetAnt {
        ant_id: row.get("ant_id"),
        ant_content: row.get("ant_content"),
    }
}

impl TweetsDao {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        TweetsDao { database: db }
    }

    pub async fn mark_scheduled_tweet_tweeted(
        &self,
        scheduled_tweet: Id,
    ) -> Result<(), anyhow::Error> {
        let mut db = self.database.lock().await;

        let t = db.transaction().await?;

        t.execute(
            "update
            scheduled_tweet
        where
            scheduled_tweet_id = $1
        set
            tweeted_at = now(),
            is_tweeted = true
        limit 1",
            &[&scheduled_tweet.0],
        )
        .await?;

        t.commit().await?;

        return Ok(());
    }

    pub async fn get_next_scheduled_tweet(&self) -> Result<Option<ScheduledTweet>, anyhow::Error> {
        let db = self.database.lock().await;

        // The interval needs to be subtracted by a day, so that the tweeter at midnight will see the tweet
        // of "today" as being valid. So placing the data at noon always and subtracting a day works.
        let scheduled_tweet_rows = db
            .query(
                "select
                scheduled_tweet_id, scheduled_at, scheduled_by, tweet_prefix, tweet_suffix
            from
                scheduled_tweet
            where
                scheduled_at >= now() - interval '1 day' and
                is_tweeted = false
            order by scheduled_at asc
            limit 1",
                &[],
            )
            .await?;

        let o_scheduled_tweet_row = scheduled_tweet_rows.first();
        if o_scheduled_tweet_row.is_none() {
            return Ok(None);
        }

        let scheduled_tweet_row = o_scheduled_tweet_row.unwrap();

        let scheduled_tweet_id: Id = scheduled_tweet_row.get("scheduled_tweet_id");
        let user_id: UserId = scheduled_tweet_row.get("scheduled_by");

        let user_name: String = db
            .query(
                "select user_name from registered_user where user_id = $1 limit 1",
                &[&user_id.0],
            )
            .await?
            .first()
            .expect("no user found")
            .get("user_name");

        let tweet_ants: Vec<TweetAnt> = db
                .query("select
            scheduled_tweet.scheduled_tweet_id, scheduled_tweet_ant.ant_id, ant_release.ant_content
        from
            scheduled_tweet left join scheduled_tweet_ant on scheduled_tweet.scheduled_tweet_id = scheduled_tweet_ant.scheduled_tweet_id
                            left join ant_release on scheduled_tweet_ant.ant_id = ant_release.ant_id
        where
            scheduled_tweet.scheduled_tweet_id = $1::uuid
            ", &[&scheduled_tweet_id.0])
                .await?
                .into_iter()
                .map(|ant_row: Row| row_to_tweet_ant(ant_row))
                .collect();

        return Ok(Some(row_to_scheduled_tweet(
            scheduled_tweet_row,
            user_name,
            tweet_ants,
        )));
    }
}
