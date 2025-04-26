pub use super::lib::Id as UserId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use chrono::{DateTime, Utc};
use double_map::DHashMap;
use futures::future;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
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
    users: DHashMap<UserId, String, Box<User>>,
}

#[async_trait::async_trait]
impl DaoTrait<UsersDao, User> for UsersDao {
    async fn new(db: Arc<Mutex<Database>>) -> Result<UsersDao, anyhow::Error> {
        let rows = db
            .lock()
            .await
            .query(
                "select user_id, user_name, user_phone_number, user_joined from registered_user;",
                &[],
            )
            .await?;

        let users_list = future::join_all(rows.iter().map(|row| {
            let user_id: UserId = row.get("user_id");
            let in_db = db.clone();
            async move {
                User {
                    user_id: row.get("user_id"),
                    username: row.get("user_name"),
                    phone_number: row.get("user_phone_number"),
                    joined: row.get("user_joined"),
                    emails: construct_emails_for_user(in_db, user_id).await,
                }
            }
        }))
        .await;

        let mut users = DHashMap::<UserId, String, Box<User>>::new();
        for user in users_list {
            users.insert(user.user_id, user.username.clone(), Box::new(user.clone()));
        }

        Ok(UsersDao {
            database: db,
            users,
        })
    }

    async fn get_one_by_id(&self, user_id: &UserId) -> Option<&User> {
        Some(self.users.get_key1(user_id)?)
    }

    async fn get_one_by_id_mut(&mut self, user_id: &UserId) -> Option<&mut User> {
        Some(self.users.get_mut_key1(user_id)?)
    }

    async fn get_one_by_name(&self, user_name: &str) -> Option<&User> {
        Some(self.users.get_key2(user_name)?)
    }

    async fn get_one_by_name_mut(&mut self, user_name: &str) -> Option<&mut User> {
        Some(self.users.get_mut_key2(user_name)?)
    }

    async fn get_all(&self) -> Vec<&User> {
        self.users
            .values()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<&User>>()
    }

    async fn get_all_mut(&mut self) -> Vec<&mut User> {
        self.users
            .values_mut()
            .map(std::convert::AsMut::as_mut)
            .collect::<Vec<&mut User>>()
    }
}

impl UsersDao {
    pub async fn create_user(
        &mut self,
        username: String,
        phone_number: String,
        emails: Vec<String>,
    ) -> Option<&User> {
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

        let user = Box::new(User {
            username,
            emails,
            phone_number,
            user_id: UserId(user_id),
            joined: chrono::offset::Utc::now(),
        });

        self.users
            .insert(user.user_id, user.username.clone(), user.clone())?
            .ok()?;
        let user = self.users.get_key1(&user.user_id)?.as_ref();
        t.commit().await.ok()?;

        return Some(user);
    }

    pub async fn add_email_to_user(&mut self, user_id: UserId, email: String) -> Option<&User> {
        debug!("IN DAO");
        let res_affected = self
            .database
            .lock()
            .await
            .execute(
                "insert into registered_user_email (user_id, user_email) values ($1::uuid, $2) limit 1",
                &[&user_id.0, &email],
            )
            .await;
        if res_affected.is_err() {
            debug!("DATABASE ERROR: {}", res_affected.unwrap_err());
            return None;
        }

        let affected = res_affected.unwrap();
        if affected != 1 {
            debug!("SQL insert for user email failed!");
            return None;
        }
        debug!("Here!");

        let user = self.users.get_mut_key1(&user_id)?;
        user.emails.push(email);

        debug!("All users: {:?}", user);
        Some(user)
    }

    pub async fn get_one_by_email(&self, email: &str) -> Option<&User> {
        self.get_all()
            .await
            .iter()
            .map(|&user| user)
            .find(|&user| user.emails.contains(&String::from(email)))
    }

    pub async fn get_one_by_phone_number(&self, phone_number: &str) -> Option<&User> {
        self.get_all()
            .await
            .iter()
            .map(|&user| user)
            .find(|&user| user.phone_number == phone_number)
    }
}
