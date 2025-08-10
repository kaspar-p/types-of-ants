use std::{fmt::Display, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{dao::db::Database, users::UserId};

pub struct WebActionsDao {
    database: Arc<Mutex<Database>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WebAction {
    #[serde(rename = "visit")]
    Visit,

    #[serde(rename = "click")]
    Click,

    #[serde(rename = "hover")]
    Hover,
}

impl Display for WebAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Click => f.write_str("click"),
            Self::Hover => f.write_str("hover"),
            Self::Visit => f.write_str("visit"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WebTargetType {
    #[serde(rename = "page")]
    Page,

    #[serde(rename = "button")]
    Button,
}

impl Display for WebTargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Page => f.write_str("page"),
            Self::Button => f.write_str("button"),
        }
    }
}

impl WebActionsDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> Result<WebActionsDao, anyhow::Error> {
        Ok(WebActionsDao { database: db })
    }

    pub async fn new_action(
        &mut self,
        actor_token: Uuid,
        actor_id: &UserId,
        action: &WebAction,
        target_type: &WebTargetType,
        target: &str,
    ) -> Result<(), anyhow::Error> {
        self.database
            .lock()
            .await
            .execute(
                "
        insert into web_action
          (actor_token, actor_user, web_action, web_target_type, web_target)
        values
          ($1, $2, $3, $4, $5)
        ",
                &[
                    &actor_token,
                    &actor_id.0,
                    &format!("{}", action),
                    &format!("{}", target_type),
                    &target,
                ],
            )
            .await?;

        Ok(())
    }
}
