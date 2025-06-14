use crate::sms::SmsSender;

use super::err::AntOnTheWebError;
use ant_data_farm::{users::User, verifications::VerificationResult, AntDataFarmClient};
use chrono::Duration;
use rand::{distr::SampleString, rngs::StdRng};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationStatus {
    pub not_verified: Vec<VerificationMethod>,
    pub verified: Vec<VerificationMethod>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    if !verifications
        .is_phone_number_verified(&user.user_id, &user.phone_number)
        .await?
    {
        not_verified.push(VerificationMethod::Phone(user.phone_number.clone()));
    } else {
        verified.push(VerificationMethod::Phone(user.phone_number.clone()));
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

/// Send a verification message to the user's phone number.
/// Assumes that there are no previous verification requests out, it is invalid to have more than 1 at a time.
pub async fn send_phone_verification_code(
    dao: &AntDataFarmClient,
    sms: &dyn SmsSender,
    rng: &mut StdRng,
    user: &User,
) -> Result<(), AntOnTheWebError> {
    let mut write_verifications = dao.verifications.write().await;
    let dist = rand::distr::Alphanumeric;
    let otp = "ANT-".to_string() + &dist.sample_string(rng, 5).to_lowercase();

    info!("Starting phone number verification for {}", &user.user_id);
    let verification = write_verifications
        .start_phone_number_verification(
            &user.user_id,
            &user.phone_number,
            Duration::minutes(5),
            &otp,
        )
        .await?;

    info!("Sending one-time password: {otp}");

    let content = format!("[typesofants.org] your one-time code is: {otp}");

    let send_id = sms.send_msg(&user.phone_number, &content).await?;
    write_verifications
        .update_phone_number_verification_with_send_id(&verification, &send_id)
        .await?;

    info!("Phone verification started, waiting user's response.");

    Ok(())
}

pub async fn resend_phone_verification_code(
    dao: &AntDataFarmClient,
    sms: &dyn SmsSender,
    rng: &mut StdRng,
    user: &User,
) -> Result<(), AntOnTheWebError> {
    dao.verifications
        .write()
        .await
        .cancel_outstanding_phone_number_verifications(&user.user_id, &user.phone_number)
        .await?;

    return send_phone_verification_code(dao, sms, rng, user).await;
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "problem")]
pub enum VerificationState {
    #[serde(rename = "verified")]
    Verified,
    #[serde(rename = "outOfAttempts")]
    OutOfAttempts,
    #[serde(rename = "hasMoreAttempts")]
    HasMoreAttempts,
    #[serde(rename = "noVerificationFound")]
    NoVerificationFound,
}

/// Based on the received phone verification request, return an HTTP response.
/// Ensures that if the user has attempted too many times, the attempt is marked as cancelled
/// so that the user can "resend".
///
/// Requires the user to be authenticated for this request to ensure no one is spoofing verification
/// attempts. The API that runs this code should allow callers that have not finished their 2FA
/// verification, that's what this is for.
pub async fn receive_phone_verification_code(
    dao: &AntDataFarmClient,
    user: &User,
    otp_attempt: &str,
) -> Result<VerificationState, AntOnTheWebError> {
    let mut write_verifications = dao.verifications.write().await;

    let verified = write_verifications
        .attempt_phone_number_verification(&user.phone_number, &otp_attempt)
        .await?;

    match verified {
        VerificationResult::Success => Ok(VerificationState::Verified),
        VerificationResult::NoVerificationFound => Ok(VerificationState::NoVerificationFound),
        VerificationResult::Failed { attempts } => {
            if attempts >= 5 {
                info!("Too many attempts, cancelling verifications");
                write_verifications
                    .cancel_outstanding_phone_number_verifications(
                        &user.user_id,
                        &user.phone_number,
                    )
                    .await?;

                return Ok(VerificationState::OutOfAttempts);
            } else {
                return Ok(VerificationState::HasMoreAttempts);
            }
        }
    }
}
