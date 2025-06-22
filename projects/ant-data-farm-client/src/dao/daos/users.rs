pub use super::lib::Id as UserId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use futures::future;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Email(String);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct User {
    #[serde(rename = "userId")]
    pub user_id: UserId,

    pub username: String,

    #[serde(rename = "phoneNumbers")]
    pub phone_numbers: Vec<String>,

    #[serde(skip)]
    pub password_hash: String,

    pub emails: Vec<String>,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub joined: DateTime<Utc>,
}

async fn construct_emails_for_user(db: &Arc<Mutex<Database>>, user_id: &UserId) -> Vec<String> {
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

async fn construct_phone_numbers_for_user(
    db: &Arc<Mutex<Database>>,
    user_id: &UserId,
) -> Vec<String> {
    db.lock()
        .await
        .query(
            "select phone_number from registered_user_phone_number where user_id = $1;",
            &[&user_id.0],
        )
        .await
        .unwrap_or_else(|_| panic!("Failed to get user phone number data!"))
        .iter()
        .map(|row| row.get("phone_number"))
        .collect::<Vec<String>>()
}

pub struct UsersDao {
    database: Arc<Mutex<Database>>,
}

fn row_to_user(user_row: &Row, emails: Vec<String>, phone_numbers: Vec<String>) -> User {
    User {
        user_id: user_row.get("user_id"),
        username: user_row.get("user_name"),
        password_hash: user_row.get("password_hash"),
        joined: user_row.get("user_joined"),
        phone_numbers,
        emails,
    }
}

#[async_trait::async_trait]
impl DaoTrait<UsersDao, User> for UsersDao {
    async fn new(db: Arc<Mutex<Database>>) -> Result<UsersDao, anyhow::Error> {
        Ok(UsersDao { database: db })
    }

    async fn get_one_by_id(&self, user_id: &UserId) -> Result<Option<User>, anyhow::Error> {
        let binding = self
            .database
            .lock()
            .await
            .query(
                "
        select
            user_id,
            user_name,
            user_joined,
            password_hash
        from
            registered_user
        where
            user_id = $1
        limit 1;",
                &[&user_id.0],
            )
            .await?;

        let row = binding.first().map(|row: &Row| async move {
            let user_id: UserId = row.get("user_id");
            let in_db = self.database.clone();
            let emails = construct_emails_for_user(&in_db, &user_id).await;
            let phone_numbers = construct_phone_numbers_for_user(&in_db, &user_id).await;
            row_to_user(row, emails, phone_numbers)
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
                "select user_id, user_name, user_joined, password_hash from registered_user;",
                &[],
            )
            .await?;

        Ok(future::join_all(rows.iter().map(|row| async move {
            let user_id: UserId = row.get("user_id");
            let in_db = self.database.clone();
            let emails = construct_emails_for_user(&in_db, &user_id).await;
            let phone_numbers = construct_phone_numbers_for_user(&in_db, &user_id).await;

            row_to_user(row, emails, phone_numbers)
        }))
        .await)
    }
}

fn make_password_hash(password: &str) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    // Step 1: Hash the password using the salt
    info!("Hashing password");
    let phc: String = match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(phc) => {
            info!("Password hashed successfully");
            phc.to_string()
        }
        Err(e) => {
            debug!("Hashing password failed: {}", e);
            return Err(anyhow::Error::msg(e.to_string()));
        }
    };

    // Step 2: Sanity check verify works
    info!("Running sanity password check");
    if !verify_password_hash(password, phc.as_str())? {
        debug!("password self-verification failed");
        return Err(anyhow::Error::msg("sanity test self-verification failed!"));
    }

    return Ok(phc);
}

pub fn verify_password_hash(
    password_attempt: &str,
    db_password: &str,
) -> Result<bool, anyhow::Error> {
    // Step: Verify attempt with stored PHC string
    let argon2 = Argon2::default();

    debug!("Parsing stored password as PHC formatted string...");
    let phc = match PasswordHash::new(db_password) {
        Ok(phc) => phc,
        Err(e) => {
            debug!("Stored password was not PHC formatted string: {}", e);
            return Err(anyhow::Error::msg(e.to_string()));
        }
    };

    debug!("Verifying hash...");
    match argon2.verify_password(password_attempt.as_bytes(), &phc) {
        Err(e) => {
            debug!("hash verification failed: {}", e);
            return Ok(false);
        }
        Ok(()) => {
            return Ok(true);
        }
    }
}

impl UsersDao {
    /// Create a user in the database, the user_name, phone_number, or email should not already be taken
    /// or else the transaction will fail.
    pub async fn create_user(
        &mut self,
        username: String,
        password: String,
        role: String,
    ) -> Result<User, anyhow::Error> {
        info!("Creating user '{}'", username);

        let mut db = self.database.lock().await;
        let t = db.transaction().await?;

        let password_hash = make_password_hash(&password)?;

        let role_id: Uuid = t
            .query(
                "select role_id from user_role where role_name = $1 limit 1;",
                &[&role],
            )
            .await?
            .first()
            .expect(format!("Could not find ID for role {role}").as_str())
            .get("role_id");

        t.execute(
            "
        insert into registered_user
            (user_name, password_hash, role_id)
        values ($1, $2, $3::uuid);",
            &[&username, &password_hash, &role_id],
        )
        .await?;

        let user_id: Uuid = t
            .query(
                "select user_id from registered_user where user_name = $1 limit 1;",
                &[&username],
            )
            .await?
            .first()
            .expect("No user_id for user we just created.")
            .get("user_id");

        let user = User {
            username: username.to_string(),
            emails: vec![],
            phone_numbers: vec![],
            password_hash,
            user_id: UserId(user_id),
            joined: chrono::offset::Utc::now(),
        };

        t.commit().await?;

        return Ok(user);
    }

    pub async fn overwrite_user_password(
        &mut self,
        user_id: &UserId,
        new_password: &str,
    ) -> Result<(), anyhow::Error> {
        let db = self.database.lock().await;
        let password_hash = make_password_hash(&new_password)?;

        db.query_one(
            "
        update registered_user
        set
            password_hash = $1,
            updated_at = now()
        where
            user_id = $2::uuid
        returning user_id
        ",
            &[&password_hash, &user_id.0],
        )
        .await?;

        Ok(())
    }

    pub async fn add_phone_number_to_user(
        &mut self,
        user_id: &UserId,
        phone_number: &str,
    ) -> Result<(), anyhow::Error> {
        let res_affected = self
            .database
            .lock()
            .await
            .execute(
                "
                insert into
                registered_user_phone_number
                    (user_id, phone_number)
                values
                    ($1::uuid, $2)
                limit 1",
                &[&user_id.0, &phone_number],
            )
            .await?;

        if res_affected != 1 {
            debug!("SQL insert for user email failed!");
            return Err(anyhow::Error::msg("More than 1 affected"));
        }
        Ok(())
    }

    pub async fn add_email_to_user(
        &mut self,
        user_id: &UserId,
        email: &str,
    ) -> Result<(), anyhow::Error> {
        let res_affected = self
            .database
            .lock()
            .await
            .execute(
                "
                insert into
                registered_user_email
                    (user_id, user_email)
                values
                    ($1::uuid, $2)
                limit 1",
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
            .find(|user| user.emails.contains(&email.to_string()));
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
            .find(|user| user.phone_numbers.contains(&phone_number.to_string()));

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
