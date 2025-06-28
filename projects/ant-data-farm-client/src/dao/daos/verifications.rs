pub use super::lib::Id as UserId;
use crate::{dao::db::Database, users::verify_password_hash};
use chrono::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

pub struct VerificationsDao {
    database: Arc<Mutex<Database>>,
}

pub enum VerificationResult {
    /// The user `user_id` has been verified.
    Success { user_id: UserId },

    /// No such verification was in the database
    NoVerificationFound,

    /// The verification attempt was wrong, current count is `attempts`
    Failed { attempts: i32 },
}

impl VerificationsDao {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        VerificationsDao { database: db }
    }

    pub async fn is_phone_number_verified(
        &self,
        user_id: &UserId,
        phone_number: &str,
    ) -> Result<bool, anyhow::Error> {
        let verifications = self
            .database
            .lock()
            .await
            .query(
                "
        select
            verification_id
        from
            verification_attempt
        where
            user_id = $1::uuid and
            verification_method = 'phone' and
            unique_key = $2 and
            is_verified = true
        ",
                &[&user_id.0, &phone_number],
            )
            .await?;

        Ok(verifications.len() == 1)
    }

    pub async fn is_email_verified(
        &self,
        user_id: &UserId,
        email: &str,
    ) -> Result<bool, anyhow::Error> {
        let verifications = self
            .database
            .lock()
            .await
            .query(
                "
        select
            verification_id
        from
            verification_attempt
        where
            user_id = $1::uuid and
            verification_method = 'email' and
            unique_key = $2 and
            is_verified = true
        ",
                &[&user_id.0, &email],
            )
            .await?;

        Ok(verifications.len() == 1)
    }

    /// Cancel all outstanding phone number verifications for a phone number.
    /// Phone numbers are unique, so they only belong to a single user.
    /// This might be done when they click "resend".
    async fn cancel_outstanding_verifications(
        &mut self,
        method: &str,
        identifier: &str,
    ) -> Result<(), anyhow::Error> {
        let mut db = self.database.lock().await;
        let t = db.transaction().await?;

        let rows = t
            .execute(
                "
            update verification_attempt
            set
                is_cancelled = true,
                cancelled_at = now()
            where
                verification_method = $1 and
                unique_key = $2 and
                is_verified = false and
                now() <= created_at + (expiration_seconds * interval '1 second')
            ",
                &[&method, &identifier],
            )
            .await?;

        if rows > 1 {
            t.rollback().await?;
            return Err(anyhow::Error::msg(format!(
                "Cancelling outstanding {method} verifications updated too many {identifier}"
            )));
        }

        t.commit().await?;

        Ok(())
    }

    /// Cancel all outstanding phone number verifications for a phone number.
    /// Phone numbers are unique, so they only belong to a single user.
    /// This might be done when they click "resend".
    pub async fn cancel_outstanding_phone_number_verifications(
        &mut self,
        phone_number: &str,
    ) -> Result<(), anyhow::Error> {
        self.cancel_outstanding_verifications("phone", phone_number)
            .await
    }

    /// Cancel all outstanding email verifications for an email address.
    /// Emails are unique, so they only belong to a single user.
    /// This might be done when they click "resend".
    pub async fn cancel_outstanding_email_verifications(
        &mut self,
        email: &str,
    ) -> Result<(), anyhow::Error> {
        self.cancel_outstanding_verifications("email", email).await
    }

    /// When a user signs up or changes their phone number, create a verification attempt row
    /// in the database for this user and phone number.
    ///
    /// Assumes that there are no ongoing verifications, cancel others before beginning this one.
    ///
    /// Returns the Verification ID.
    async fn start_verification(
        &mut self,
        user_id: &UserId,
        method: &str,
        identifier: &str,
        expiration: Duration,
        otp: &str,
    ) -> Result<Uuid, anyhow::Error> {
        let mut db = self.database.lock().await;
        let t = db.transaction().await?;

        let verification_id: Uuid = t
            .query_one(
                "
        insert into verification_attempt
            (user_id, unique_key, verification_method, expiration_seconds, one_time_code)
        values
            ($1::uuid, $2, $3, $4, $5)
        returning verification_id
        ",
                &[
                    &user_id.0,
                    &identifier,
                    &method,
                    &expiration.num_seconds(),
                    &otp,
                ],
            )
            .await?
            .get("verification_id");

        t.commit().await?;

        return Ok(verification_id);
    }

    /// When a user signs up or changes their phone number, create a verification attempt row
    /// in the database for this user and phone number.
    ///
    /// Assumes that there are no ongoing verifications, cancel others before beginning this one.
    ///
    /// Returns the Verification ID.
    pub async fn start_phone_number_verification(
        &mut self,
        user_id: &UserId,
        phone_number: &str,
        expiration: Duration,
        otp: &str,
    ) -> Result<Uuid, anyhow::Error> {
        self.start_verification(user_id, "phone", phone_number, expiration, otp)
            .await
    }

    /// When a user signs up or changes their phone number, create a verification attempt row
    /// in the database for this user and phone number.
    ///
    /// Assumes that there are no ongoing verifications, cancel others before beginning this one.
    ///
    /// Returns the Verification ID.
    pub async fn start_email_verification(
        &mut self,
        user_id: &UserId,
        email: &str,
        expiration: Duration,
        otp: &str,
    ) -> Result<Uuid, anyhow::Error> {
        self.start_verification(user_id, "email", email, expiration, otp)
            .await
    }

    /// For a verification request, once the request is sent and there is some unique ID to associate
    /// that came from the SMS/Email provider, save that into the DB.
    pub async fn update_verification_with_send_id(
        &mut self,
        verification_attempt_id: &Uuid,
        send_id: &str,
    ) -> Result<(), anyhow::Error> {
        self.database
            .lock()
            .await
            .execute(
                "
        update verification_attempt
        set send_id = $2
        where verification_id = $1",
                &[&verification_attempt_id, &send_id],
            )
            .await?;

        Ok(())
    }

    async fn attempt_verification(
        &mut self,
        method: &str,
        identifier: &str,
        attempt: &str,
    ) -> Result<VerificationResult, anyhow::Error> {
        let mut db = self.database.lock().await;
        let t = db.transaction().await?;

        let verification = t
            .query_opt(
                "
        select
            verification_id, one_time_code
        from
            verification_attempt
        where
            verification_method = $1 and
            unique_key = $2 and
            is_cancelled = false and
            is_verified = false and
            now() <= created_at + (expiration_seconds * interval '1 second')
        ",
                &[&method, &identifier],
            )
            .await?;

        let row = match verification {
            None => {
                debug!("No unexpired verification for {identifier} found.");
                return Ok(VerificationResult::NoVerificationFound);
            }
            Some(row) => row,
        };

        let otp: String = row.get("one_time_code");
        let verification_id: Uuid = row.get("verification_id");

        info!("comparing attempt to {otp} for {verification_id}");

        if verify_password_hash(&attempt, &otp)? {
            info!("otp attempt succeeded, marking {verification_id} as verified");
            let row = t
                .query_one(
                    "
            update verification_attempt
            set
                verification_attempts = verification_attempts + 1,
                is_verified = true,
                verified_at = now()
            where
                verification_id = $1::uuid
            returning user_id
            ",
                    &[&verification_id],
                )
                .await?;

            let user_id: UserId = row.get("user_id");

            t.commit().await?;

            return Ok(VerificationResult::Success { user_id });
        } else {
            info!("otp attempt failed, incrementing row of {verification_id}");
            let attempt_row = t
                .query_one(
                    "
            update verification_attempt
            set
                verification_attempts = verification_attempts + 1
            where
                verification_id = $1::uuid
            returning verification_attempts
            ",
                    &[&verification_id],
                )
                .await?;

            let attempts: i32 = attempt_row.get("verification_attempts");

            t.commit().await?;

            return Ok(VerificationResult::Failed { attempts });
        }
    }

    /// When the user has attempted to verify their phone number, check whether the one-time pad
    /// actually matches what we expect.
    ///
    /// If it is, then updates the row to mark the phone number as verified, and when the verification
    /// happened. Returns `true`.
    ///
    /// If the attempt does not match, returns `false` and the application should ask the user to retry.
    /// Also returns false if there was no verification that matched for those details, or might be expired.
    pub async fn attempt_phone_number_verification(
        &mut self,
        phone_number: &str,
        attempt: &str,
    ) -> Result<VerificationResult, anyhow::Error> {
        self.attempt_verification("phone", phone_number, attempt)
            .await
    }

    /// When the user has attempted to verify their email, check whether the one-time pad
    /// actually matches what we expect.
    ///
    /// If it is, then updates the row to mark the phone number as verified, and when the verification
    /// happened. Returns `true`.
    ///
    /// If the attempt does not match, returns `false` and the application should ask the user to retry.
    /// Also returns false if there was no verification that matched for those details, or might be expired.
    pub async fn attempt_email_verification(
        &mut self,
        email: &str,
        attempt: &str,
    ) -> Result<VerificationResult, anyhow::Error> {
        self.attempt_verification("email", email, attempt).await
    }
}
