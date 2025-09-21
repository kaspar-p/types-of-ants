use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::{
    dao::db::Database,
    users::{make_password_hash, verify_password_hash, UserId},
};

pub struct ApiTokensDao {
    database: Arc<Mutex<Database>>,
}

impl ApiTokensDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> Result<Self, anyhow::Error> {
        Ok(Self { database: db })
    }

    pub async fn register_api_token(
        &mut self,
        user_id: &UserId,
        api_token: &str,
    ) -> Result<(), anyhow::Error> {
        let token_hash = make_password_hash(api_token)?;

        self.database
            .lock()
            .await
            .get()
            .await?
            .execute(
                "
            insert into api_token
              (user_id, api_token_hash)
            values
              ($1, $2)",
                &[&user_id.0, &token_hash],
            )
            .await?;

        Ok(())
    }

    /// Returns Some(user_id) if the username has that api_token, or Some(None) if not.
    pub async fn verify_token_user(
        &self,
        username: &str,
        api_token: &str,
    ) -> Result<Option<UserId>, anyhow::Error> {
        let users = self
            .database
            .lock()
            .await
            .get()
            .await?
            .query(
                "select
                  api_token.user_id, api_token.api_token_hash
                from api_token
                  join registered_user on registered_user.user_id = api_token.user_id
                where user_name = $1",
                &[&username],
            )
            .await?;
        info!("Matched {} user(s)...", users.len());

        for user in users {
            if verify_password_hash(api_token, user.get("api_token_hash"))? {
                info!("Verified hash successfully for user {}", username);
                return Ok(Some(user.get("user_id")));
            } else {
                warn!("Hash was not valid for user {username}!");
            }
        }

        Ok(None)
    }
}
