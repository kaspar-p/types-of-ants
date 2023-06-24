pub use super::lib::Id as UserId;
use crate::dao::db::Database;
use chrono::{DateTime, Utc};
use futures::future;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Email(String);

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    user_id: UserId,
    username: String,
    emails: Vec<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    joined: DateTime<Utc>,
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
    users: HashMap<UserId, User>,
}

impl UsersDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> UsersDao {
        let rows = db
            .lock()
            .await
            .query(
                "select user_id, user_name, user_joined from registered_user;",
                &[],
            )
            .await
            .unwrap_or_else(|_| panic!("Fetching user data failed!"));

        let users_list = future::join_all(rows.iter().map(|row| {
            let user_id: UserId = row.get("user_id");
            let in_db = db.clone();
            return async move {
                User {
                    user_id: row.get("user_id"),
                    username: row.get("user_name"),
                    joined: row.get("user_joined"),
                    emails: construct_emails_for_user(in_db, user_id).await,
                }
            };
        }))
        .await;

        let mut users = HashMap::<UserId, User>::new();
        for user in users_list {
            users.insert(user.user_id.clone(), user.clone());
        }

        UsersDao {
            database: db,
            users,
        }
    }

    pub async fn create_user(&mut self, username: String, emails: Vec<String>) -> Option<User> {
        self.database.lock().await.query(
            "insert into registered_user (user_name) values ($1);",
            &[&username],
        );

        let user_id: Uuid = self
            .database
            .lock()
            .await
            .query(
                "select user_id from registered_user where user_name = $1",
                &[&username],
            )
            .await
            .unwrap_or_else(|_| panic!("Creating user failed!"))[0]
            .get("user_id");

        Some(User {
            username,
            emails,
            user_id: UserId(user_id),
            joined: chrono::offset::Utc::now(),
        })
    }

    pub async fn get_user(&self, user_id: UserId) -> Option<&User> {
        self.users.get(&user_id)
    }

    pub async fn get_all_users(&self) -> Vec<&User> {
        self.users
            .keys()
            .map(|k| self.users.get(k).unwrap())
            .collect()
    }
}
