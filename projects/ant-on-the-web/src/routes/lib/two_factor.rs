use crate::{
    email::{EmailError, EmailSender},
    err::{ValidationError, ValidationMessage},
    sms::{SmsError, SmsSender},
};

use super::err::AntOnTheWebError;
use ant_data_farm::{
    users::{make_password_hash, User, UserId},
    verifications::VerificationResult,
    AntDataFarmClient,
};
use chrono::Duration;
use rand::{distr::SampleString, rngs::StdRng};
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
    let verifications = dao.verifications.read().await;

    let mut verified: Vec<VerificationMethod> = vec![];
    let mut not_verified: Vec<VerificationMethod> = vec![];

    for phone_number in &user.phone_numbers {
        if !verifications
            .is_phone_number_verified(&user.user_id, &phone_number)
            .await?
        {
            not_verified.push(VerificationMethod::Phone(phone_number.clone()));
        } else {
            verified.push(VerificationMethod::Phone(phone_number.clone()));
        }
    }

    for email in &user.emails {
        if !verifications
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

/// Send a verification code to a user's email address.
/// Assumes there are no previous verification requests out, it is invalid to have more than 1 at a time.
async fn send_email_verification_code(
    dao: &AntDataFarmClient,
    email_sender: &dyn EmailSender,
    rng: &mut StdRng,
    user_id: &UserId,
    email: &str,
) -> Result<(), AntOnTheWebError> {
    let mut write_verifications = dao.verifications.write().await;
    let dist = rand::distr::Alphanumeric;

    let otp = "ant-".to_string() + &dist.sample_string(rng, 5).to_lowercase();
    let otp_hash = make_password_hash(&otp)?;

    info!("Starting email verification for {user_id} on {email} with {otp}");
    let verification = write_verifications
        .start_email_verification(&user_id, &email, Duration::minutes(5), &otp_hash)
        .await?;

    info!("Sending one-time password. Omitting logging it.");

    let subject = "your one-time code".to_string();
    let content = format!(
        "hello,

a login or sign-in request generated a one-time code: {otp}

if you did not generate this code, someone may be trying to access your account, please reset your password as soon as possible.

with love,
    the typesofants.org team"
    );

    let send_id = email_sender
        .send_email(email, subject, content)
        .await
        .map_err(|e| match e {
            EmailError::InternalServerError(e) => AntOnTheWebError::InternalServerError(Some(e)),
        })?;

    write_verifications
        .update_verification_with_send_id(&verification, &send_id)
        .await?;

    info!("Email verification started, waiting user's response.");

    Ok(())
}

pub async fn resend_email_verification_code(
    dao: &AntDataFarmClient,
    email_sender: &dyn EmailSender,
    rng: &mut StdRng,
    user_id: &UserId,
    email: &str,
) -> Result<(), AntOnTheWebError> {
    dao.verifications
        .write()
        .await
        .cancel_outstanding_email_verifications(email)
        .await?;

    return send_email_verification_code(dao, email_sender, rng, user_id, email).await;
}

/// Based on the received email verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
pub async fn receive_email_verification_code(
    dao: &AntDataFarmClient,
    email: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    let mut write_verifications = dao.verifications.write().await;

    info!("Attempting to verify 2fa attempt");
    let verified = write_verifications
        .attempt_email_verification(&email, &otp_attempt)
        .await?;

    match verified {
        VerificationResult::Success { user_id } => Ok(VerificationReceipt::Success { user_id }),
        VerificationResult::NoVerificationFound => {
            info!("No such verification found");
            return Ok(VerificationReceipt::Failed);
        }
        VerificationResult::Failed { attempts } => {
            if attempts >= 5 {
                info!("Too many attempts, cancelling verifications");
                write_verifications
                    .cancel_outstanding_email_verifications(&email)
                    .await?;
            }
            info!("two-factor attempt failed");
            return Ok(VerificationReceipt::Failed);
        }
    }
}

/// Send a verification message to the user's phone number.
/// Assumes that there are no previous verification requests out, it is invalid to have more than 1 at a time.
async fn send_phone_verification_code(
    dao: &AntDataFarmClient,
    sms: &dyn SmsSender,
    rng: &mut StdRng,
    user_id: &UserId,
    phone_number: &str,
) -> Result<(), AntOnTheWebError> {
    let mut write_verifications = dao.verifications.write().await;
    let dist = rand::distr::Alphanumeric;

    let otp = "ant-".to_string() + &dist.sample_string(rng, 5).to_lowercase();
    let otp_hash = make_password_hash(&otp)?;

    info!("Starting phone number verification for {user_id} on {phone_number} with {otp}");
    let verification = write_verifications
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

    write_verifications
        .update_verification_with_send_id(&verification, &send_id)
        .await?;

    info!("Phone verification started, waiting user's response.");

    Ok(())
}

pub async fn resend_phone_verification_code(
    dao: &AntDataFarmClient,
    sms: &dyn SmsSender,
    rng: &mut StdRng,
    user_id: &UserId,
    phone_number: &str,
) -> Result<(), AntOnTheWebError> {
    dao.verifications
        .write()
        .await
        .cancel_outstanding_phone_number_verifications(phone_number)
        .await?;

    return send_phone_verification_code(dao, sms, rng, user_id, phone_number).await;
}

pub enum VerificationReceipt {
    Success { user_id: UserId },
    Failed,
}

/// Based on the received phone verification request.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Returns true if the user is verified, false if not. The user may have more attempts or not,
/// if the return value is false.
///
/// This is a dangerous function, use only when user is authenticated or similar requirements.
pub async fn receive_phone_verification_code(
    dao: &AntDataFarmClient,
    phone_number: &str,
    otp_attempt: &str,
) -> Result<VerificationReceipt, anyhow::Error> {
    let mut write_verifications = dao.verifications.write().await;

    info!("Attempting to verify 2fa attempt");
    let verified = write_verifications
        .attempt_phone_number_verification(&phone_number, &otp_attempt)
        .await?;

    match verified {
        VerificationResult::Success { user_id } => Ok(VerificationReceipt::Success { user_id }),
        VerificationResult::NoVerificationFound => {
            info!("No such verification found");
            return Ok(VerificationReceipt::Failed);
        }
        VerificationResult::Failed { attempts } => {
            if attempts >= 5 {
                info!("Too many attempts, cancelling verifications");
                write_verifications
                    .cancel_outstanding_phone_number_verifications(&phone_number)
                    .await?;
            }
            info!("two-factor attempt failed");
            return Ok(VerificationReceipt::Failed);
        }
    }
}
