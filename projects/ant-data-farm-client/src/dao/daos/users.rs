pub use super::lib::Id as UserId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use chrono::{DateTime, Utc};
use futures::future;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Email(String);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub user_id: UserId,
    pub username: String,
    pub phone_number: String,
    pub emails: Vec<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub joined: DateTime<Utc>,
}

async fn construct_emails_for_user(db: Arc<Mutex<Database>>, user_id: UserId) -> Vec<String> {
    db.lock()
        .await
        .query(
            "select user_email from registered_user_email where user_id = $1;",
            &[&user_id.0],
        )
        .await
        .unwrap_or_else(|_| panic!("Failed to get user email data!"))
        .iter()
        .map(|email_row| email_row.get("user_email"))
        .collect::<Vec<String>>()
}

pub struct UsersDao {
    database: Arc<Mutex<Database>>,
}

fn row_to_user(user_row: &Row, emails: Vec<String>) -> User {
    User {
        user_id: user_row.get("user_id"),
        username: user_row.get("user_name"),
        phone_number: user_row.get("user_phone_number"),
        emails,
        joined: user_row.get("user_joined"),
    }
}

#[async_trait::async_trait]
impl DaoTrait<UsersDao, User> for UsersDao {
    async fn new(db: Arc<Mutex<Database>>) -> Result<UsersDao, anyhow::Error> {
        Ok(UsersDao { database: db })
    }

    async fn get_one_by_id(&self, user_id: &UserId) -> Result<Option<User>, anyhow::Error> {
        let binding = self.database
        .lock()
        .await
        .query(
            "select user_id, user_name, user_phone_number, user_joined from registered_user where user_id = $1 limit 1;",
            &[&user_id.0],
        )
        .await?;

        let row = binding.first().map(|row: &Row| async move {
            let user_id: UserId = row.get("user_id");
            let in_db = self.database.clone();
            let emails = construct_emails_for_user(in_db, user_id).await;
            row_to_user(row, emails)
        });

        match row {
            Some(user) => Ok(Some(user.await)),
            None => Ok(None),
        }
    }

    async fn get_all(&self) -> Result<Vec<User>, anyhow::Error> {
        let rows = self
            .database
            .lock()
            .await
            .query(
                "select user_id, user_name, user_phone_number, user_joined from registered_user;",
                &[],
            )
            .await?;

        Ok(future::join_all(rows.iter().map(|row| async move {
            let user_id: UserId = row.get("user_id");
            let in_db = self.database.clone();
            let emails = construct_emails_for_user(in_db, user_id).await;

            row_to_user(row, emails)
        }))
        .await)
    }
}

impl UsersDao {
    pub async fn create_user(
        &mut self,
        username: String,
        phone_number: String,
        emails: Vec<String>,
    ) -> Option<User> {
        let mut db = self.database.lock().await;
        let t = db.transaction().await.unwrap();

        t.query(
            "
        insert into registered_user 
            (user_name, user_phone_number)
        values ($1, $2);",
            &[&username, &phone_number],
        )
        .await
        .unwrap_or_else(|e| panic!("Failed to create user: {e}"));

        let user_id: Uuid = t
            .query(
                "select user_id from registered_user where user_name = $1",
                &[&username],
            )
            .await
            .unwrap_or_else(|_| panic!("Creating user failed!"))[0]
            .get("user_id");

        let mut query = String::from(
            "
        insert into registered_user_email
            (user_id, user_email)
        values
        ",
        );
        query.push_str(
            emails
                .iter()
                .map(|e| format!("({user_id}, {e})"))
                .collect::<Vec<String>>()
                .join(",\n")
                .as_str(),
        );
        query.push_str(";");

        if let Err(e) = t.execute(&query, &[]).await {
            debug!("Email insert failed: {e}");
            t.rollback().await.expect("rollback failed");
            return None;
        }

        let user = User {
            username,
            emails,
            phone_number,
            user_id: UserId(user_id),
            joined: chrono::offset::Utc::now(),
        };

        t.commit().await.ok()?;

        return Some(user);
    }

    pub async fn add_email_to_user(
        &mut self,
        user_id: UserId,
        email: String,
    ) -> Result<(), anyhow::Error> {
        debug!("IN DAO");
        let res_affected = self
            .database
            .lock()
            .await
            .execute(
                "insert into registered_user_email (user_id, user_email) values ($1::uuid, $2) limit 1",
                &[&user_id.0, &email],
            )
            .await?;

        if res_affected != 1 {
            debug!("SQL insert for user email failed!");
            return Err(anyhow::Error::msg("More than 1 affected"));
        }
        Ok(())
    }

    pub async fn get_one_by_email(&self, email: &str) -> Result<Option<User>, anyhow::Error> {
        let user = self
            .get_all()
            .await?
            .into_iter()
            .map(|user| user)
            .find(|user| user.emails.contains(&String::from(email)));
        Ok(user)
    }

    pub async fn get_one_by_phone_number(
        &self,
        phone_number: &str,
    ) -> Result<Option<User>, anyhow::Error> {
        let user = self
            .get_all()
            .await?
            .into_iter()
            .map(|user| user)
            .find(|user| user.phone_number == phone_number);

        Ok(user)
    }

    pub async fn get_one_by_user_name(
        &self,
        username: &str,
    ) -> Result<Option<User>, anyhow::Error> {
        let user = self
            .get_all()
            .await?
            .into_iter()
            .map(|user| user)
            .find(|user| user.username == username);

        Ok(user)
    }
}
