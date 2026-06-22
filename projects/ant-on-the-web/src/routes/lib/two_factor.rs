use crate::{
    email::{EmailError, EmailSender},
    err::{ValidationError, ValidationMessage},
    sms::{SmsError, SmsSender},
};

use super::err::AntOnTheWebError;
use ant_data_farm::{
    users::{User, UserId},
    verifications::VerificationResult,
    AntDataFarmClient,
};
use ant_library::rng::{RandAdapter, Rng};
use chrono::Duration;
use rand::distr::SampleString;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationStatus {
    pub not_verified: Vec<VerificationMethod>,
    pub verified: Vec<VerificationMethod>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationMethod {
    #[serde(rename = "email")]
    Email(String),
    #[serde(rename = "phone")]
    Phone(String),
}

pub async fn user_is_two_factor_verified(
    dao: &AntDataFarmClient,
    user: &User,
) -> Result<VerificationStatus, anyhow::Error> {
    let mut verified: Vec<VerificationMethod> = vec![];
    let mut not_verified: Vec<VerificationMethod> = vec![];

    for phone_number in &user.phone_numbers {
        if !dao
            .verifications
            .is_phone_number_verified(&user.user_id, &phone_number)
            .await?
        {
            not_verified.push(VerificationMethod::Phone(phone_number.clone()));
        } else {
            verified.push(VerificationMethod::Phone(phone_number.clone()));
        }
    }

    for email in &user.emails {
        if !dao
            .verifications
            .is_email_verified(&user.user_id, &email)
            .await?
        {
            not_verified.push(VerificationMethod::Email(email.clone()));
        } else {
            verified.push(VerificationMethod::Email(email.clone()));
        }
    }

    return Ok(VerificationStatus {
        verified,
        not_verified,
    });
}

pub enum VerificationReceipt {
    Success { user_id: UserId },
    Failed,
}

/// Send a verification code to a user's email address.
/// Assumes there are no previous verification requests out, it is invalid to have more than 1 at a time.
async fn send_email_verification_code(
    dao: &AntDataFarmClient,
    email_sender: &dyn EmailSender,
    rng: &dyn Rng,
    user_id: &UserId,
    email: &str,
) -> Result<(), AntOnTheWebError> {
    let otp = "ant-".to_string()
        + &rand::distr::Alphanumeric
            .sample_string(&mut RandAdapter(rng), 5)
            .to_lowercase();
    let otp_hash = ant_library::crypto::make_password_hash(&otp).await?;

    info!("Starting email verification for {user_id} on {email} with {otp}");
    let verification = dao
        .verifications
        .start_email_verification(&user_id, &email, Duration::minutes(5), &otp_hash)
        .await?;

    info!("Sending one-time password. Omitting logging it.");

    let subject = "your one-time code".to_string();
    let content = format!(
        "hello,

a login or sign-in request generated a one-time code: {otp}

if you did not generate this code, someone may be trying to access your account, please reset your \
         password as soon as possible.

with love,
    the typesofants.org team"
    );

    let send_id = email_sender
        .send_email(email, subject, content)
        .await
        .map_err(|e| match e {
            EmailError::InternalServerError(e) => AntOnTheWebError::InternalServerError(Some(e)),
        })?;

    dao.verifications
        .update_verification_with_send_id(&verification, &send_id)
        .await?;

    info!("Email verification started, waiting user's response.");

    Ok(())
}

pub async fn resend_email_verification_code(
    dao: &AntDataFarmClient,
    email_sender: &dyn EmailSender,
    rng: &dyn Rng,
    user_id: &UserId,
    email: &str,
) -> Result<(), AntOnTheWebError> {
    dao.verifications
        .cancel_outstanding_email_verifications(email)
        .await?;

    return send_email_verification_code(dao, email_sender, rng, user_id, email).await;
}

async fn receive_email_verification_code(
    dao: &AntDataFarmClient,
    attemptor_user_id: Option<&UserId>,
    email: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    info!("Attempting to verify 2fa attempt");
    let verified = dao
        .verifications
        .attempt_email_verification(attemptor_user_id, &email, &otp_attempt)
        .await?;

    match verified {
        VerificationResult::Success {
            user_id: verified_user_id,
        } => {
            if let Some(user_id) = attemptor_user_id {
                assert_eq!(
                    *user_id, verified_user_id,
                    "2fa attempt should have the same requesting-user as receiving-user!"
                );
            }

            Ok(VerificationReceipt::Success {
                user_id: verified_user_id,
            })
        }
        VerificationResult::NoVerificationFound => {
            info!("No such verification found");
            return Ok(VerificationReceipt::Failed);
        }
        VerificationResult::Failed { attempts } => {
            if attempts >= 5 {
                info!("Too many attempts, cancelling verifications");
                dao.verifications
                    .cancel_outstanding_email_verifications(&email)
                    .await?;
            }
            info!("two-factor attempt failed");
            return Ok(VerificationReceipt::Failed);
        }
    }
}

/// Based on the received email verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
///
/// Should be used when there is a user token, e.g. initial signup (weak token) or adding new email
/// to an existing account (strong token).
pub async fn receive_email_verification_code_for_user(
    dao: &AntDataFarmClient,
    attemptor_user_id: &UserId,
    email: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    receive_email_verification_code(dao, Some(attemptor_user_id), email, otp_attempt).await
}

/// Based on the received email verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
///
/// Should ONLY be used for password reset, otherwise see [`receive_email_verification_code_for_user`].
pub async fn receive_email_verification_code_for_anyone(
    dao: &AntDataFarmClient,
    email: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    receive_email_verification_code(dao, None, email, otp_attempt).await
}

/// Send a verification message to the user's phone number.
/// Assumes that there are no previous verification requests out, it is invalid to have more than 1 at a time.
async fn send_phone_verification_code(
    dao: &AntDataFarmClient,
    sms: &dyn SmsSender,
    rng: &dyn Rng,
    user_id: &UserId,
    phone_number: &str,
) -> Result<(), AntOnTheWebError> {
    let otp = "ant-".to_string()
        + &rand::distr::Alphanumeric
            .sample_string(&mut RandAdapter(rng), 5)
            .to_lowercase();
    let otp_hash = ant_library::crypto::make_password_hash(&otp).await?;

    info!("Starting phone number verification for {user_id} on {phone_number} with {otp}");
    let verification = dao
        .verifications
        .start_phone_number_verification(&user_id, &phone_number, Duration::minutes(5), &otp_hash)
        .await?;

    info!("Sending one-time password. Omitting logging it.");
    let content = format!("[typesofants.org] your one-time code is: {otp}");

    let send_id = sms
        .send_msg(&phone_number, &content)
        .await
        .map_err(|e| match e {
            SmsError::BadPhoneNumber => AntOnTheWebError::ValidationError(ValidationError::one(
                ValidationMessage::new("phone", "Phone number cannot receive messages"),
            )),
            SmsError::InternalServerError(e) => AntOnTheWebError::InternalServerError(Some(e)),
        })?;

    dao.verifications
        .update_verification_with_send_id(&verification, &send_id)
        .await?;

    info!("Phone verification started, waiting user's response.");

    Ok(())
}

pub async fn resend_phone_verification_code(
    dao: &AntDataFarmClient,
    sms: &dyn SmsSender,
    rng: &dyn Rng,
    user_id: &UserId,
    phone_number: &str,
) -> Result<(), AntOnTheWebError> {
    dao.verifications
        .cancel_outstanding_phone_number_verifications(phone_number)
        .await?;

    return send_phone_verification_code(dao, sms, rng, user_id, phone_number).await;
}

/// Based on the received phone verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
async fn receive_phone_verification_code(
    dao: &AntDataFarmClient,
    attemptor_user_id: Option<&UserId>,
    phone_number: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    info!("Attempting to verify 2fa attempt");
    let verified = dao
        .verifications
        .attempt_phone_number_verification(attemptor_user_id, &phone_number, &otp_attempt)
        .await?;

    match verified {
        VerificationResult::Success {
            user_id: verified_user_id,
        } => {
            if let Some(user_id) = attemptor_user_id {
                assert_eq!(
                    *user_id, verified_user_id,
                    "2fa attempt should have the same requesting-user as receiving-user!"
                );
            }

            Ok(VerificationReceipt::Success {
                user_id: verified_user_id,
            })
        }
        VerificationResult::NoVerificationFound => {
            info!("No such verification found");
            return Ok(VerificationReceipt::Failed);
        }
        VerificationResult::Failed { attempts } => {
            if attempts >= 5 {
                info!("Too many attempts, cancelling verifications");
                dao.verifications
                    .cancel_outstanding_phone_number_verifications(&phone_number)
                    .await?;
            }
            info!("two-factor attempt failed");
            return Ok(VerificationReceipt::Failed);
        }
    }
}

/// Based on the received phone verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
pub async fn receive_phone_verification_code_for_user(
    dao: &AntDataFarmClient,
    user_id: &UserId,
    phone_number: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    receive_phone_verification_code(dao, Some(user_id), phone_number, otp_attempt).await
}

/// Based on the received phone verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
pub async fn receive_phone_verification_code_for_anyone(
    dao: &AntDataFarmClient,
    phone_number: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    receive_phone_verification_code(dao, None, phone_number, otp_attempt).await
}
