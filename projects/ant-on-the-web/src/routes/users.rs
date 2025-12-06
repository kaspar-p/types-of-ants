use std::str::FromStr;

use crate::{
    err::ValidationError,
    routes::lib::{
        auth::{make_auth_cookie, make_weak_auth_cookie, AUTH_COOKIE_NAME},
        err::ValidationMessage,
        two_factor,
    },
    state::{ApiRouter, ApiState, InnerApiState},
    two_factor::VerificationReceipt,
};
use ant_data_farm::users::{verify_password_hash, User, UserId};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use tracing::{debug, info};

use super::lib::{
    auth::{
        authenticate, authenticate_or_weak_matching_method, authenticate_weak, cookie_defaults,
        optional_authenticate, AuthClaims,
    },
    err::AntOnTheWebError,
    jwt::{decode_jwt, encode_jwt},
};

#[derive(Serialize, Deserialize)]
pub struct EmailRequest {
    pub email: String,
}
async fn subscribe_email(
    auth: Option<AuthClaims>,
    State(InnerApiState { dao, .. }): ApiState,
    Json(EmailRequest {
        email: unsafe_email,
    }): Json<EmailRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = optional_authenticate(auth.as_ref(), &dao).await?;

    let canonical_email = match canonicalize_email(&unsafe_email) {
        Ok(e) => e,
        Err(e) => {
            info!("email invalid {e}");
            return Err(AntOnTheWebError::Validation(ValidationError::one(
                ValidationMessage::invalid("email"),
            )));
        }
    };

    {
        let users = dao.users.read().await;

        if let Some(u) = users.get_one_by_email(&canonical_email).await? {
            if u != user {
                return Err(AntOnTheWebError::ConflictError("Already subscribed!"));
            }
        }
    }

    if user.emails.contains(&canonical_email) {
        return Err(AntOnTheWebError::ConflictError("Already subscribed!"));
    }

    let mut user_write = dao.users.write().await;
    user_write
        .add_email_to_user(&user.user_id, &canonical_email)
        .await?;

    return Ok((StatusCode::OK, "Subscribed!"));
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetUserResponse {
    pub user: User,
}
async fn get_user_by_name(
    auth: AuthClaims,
    path: Option<Path<String>>,
    State(InnerApiState { dao, .. }): ApiState,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    // AuthN
    let user = authenticate(&auth, &dao).await?;

    if path.is_none() {
        return Ok((
            StatusCode::OK,
            Json(GetUserResponse { user }).into_response(),
        ));
    }

    let user_name = path.unwrap().0;

    // AuthZ
    if user.username != user_name {
        return Err(AntOnTheWebError::AccessDenied(Some(
            user.user_id.to_string(),
        )));
    }
    info!("Granted access to {}", user.user_id);

    let users = dao.users.read().await;
    let user = users.get_one_by_user_name(&user_name).await?.unwrap();
    return Ok((
        StatusCode::OK,
        Json(GetUserResponse { user }).into_response(),
    ));
}

#[derive(Serialize, Deserialize)]
pub struct ChangeUsernameRequest {
    pub username: String,
}

async fn change_username(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<ChangeUsernameRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = authenticate(&auth, &dao).await?;

    let validations = validate_username(&req.username);
    if validations.len() > 0 {
        return Err(AntOnTheWebError::Validation(ValidationError::many(
            validations,
        )));
    }

    let mut users_write = dao.users.write().await;

    let other = users_write.get_one_by_user_name(&req.username).await?;

    match other {
        Some(other) => {
            if other.user_id == user.user_id {
                return Err(AntOnTheWebError::Validation(ValidationError::one(
                    ValidationMessage::new("username", "Must not be your current username."),
                )));
            } else {
                return Err(AntOnTheWebError::ConflictError("Username already in use."));
            }
        }
        None => {
            users_write
                .change_username(&user.user_id, &req.username)
                .await?;

            Ok((StatusCode::OK, "Username changed."))
        }
    }
}

async fn logout(
    auth: AuthClaims,
    cookies: Cookies,
    State(InnerApiState { dao, .. }): ApiState,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    authenticate(&auth, &dao).await?;

    let mut cookie_expiration = cookie_defaults(AUTH_COOKIE_NAME, "".to_string()).build();
    cookie_expiration.make_removal();

    cookies.add(cookie_expiration);

    return Ok((StatusCode::OK, "Logout successful."));
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LoginMethod {
    #[serde(rename = "username")]
    Username(String),
    #[serde(rename = "email")]
    Email(String),
    #[serde(rename = "phoneNumber")]
    Phone(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginRequest {
    pub method: LoginMethod,
    pub password: String,
}

fn canonicalize_phone_number(phone_number: &str) -> Result<String, anyhow::Error> {
    Ok(phonenumber::parse(None, phone_number)?.format().to_string())
}

fn canonicalize_email(email: &str) -> Result<String, anyhow::Error> {
    Ok(email_address::EmailAddress::from_str(email)?
        .as_str()
        .to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "userId")]
    pub user_id: UserId,

    #[serde(rename = "accessToken")]
    pub access_token: String,
}
async fn login(
    cookies: Cookies,
    State(InnerApiState { dao, .. }): ApiState,
    Json(login_request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    // AuthN: This is the starting entry point for AuthN, so we don't need any auth claims here

    let users = dao.users.read().await;
    let user: Result<Option<User>, ValidationMessage> = match login_request.method {
        LoginMethod::Email(email) => match canonicalize_email(email.as_str()) {
            Err(e) => {
                info!("Field method.email invalid: {}", e);
                Err(ValidationMessage::invalid("method.email"))
            }
            Ok(email) => Ok(users.get_one_by_email(&email).await?),
        },
        LoginMethod::Phone(phone) => match canonicalize_phone_number(&phone) {
            Err(e) => {
                info!("Field method.phone invalid: {}", e);
                Err(ValidationMessage::invalid("method.phone"))
            }
            Ok(phone) => Ok(users.get_one_by_phone_number(&phone).await?),
        },
        LoginMethod::Username(username) => Ok(users.get_one_by_user_name(&username).await?),
    };

    let user = match user {
        Err(v) => {
            return Err(AntOnTheWebError::Validation(ValidationError::one(v)));
        }
        Ok(None) => {
            info!("No user found");
            return Err(AntOnTheWebError::AccessDenied(None));
        }
        Ok(Some(user)) => user,
    };

    if !verify_password_hash(
        login_request.password.as_str(),
        &user.password_hash.as_str(),
    )? {
        info!("Password invalid");
        return Err(AntOnTheWebError::AccessDenied(None));
    }

    // AuthN: Make weak auth cookie because the user still has to 2fa verify. Strong tokens are vended
    // only by the 2fa verification endpoints
    debug!("Password verified, generating jwt token...");
    let (jwt, cookie) = make_weak_auth_cookie(user.user_id.clone())?;

    cookies.add(cookie);

    return Ok((
        StatusCode::OK,
        Json(LoginResponse {
            user_id: user.user_id.clone(),
            access_token: jwt,
        }),
    )
        .into_response());
}

#[derive(Serialize, Deserialize)]
pub enum VerificationSubmission {
    #[serde(rename = "email")]
    Email { email: String, otp: String },

    #[serde(rename = "phone")]
    Phone {
        #[serde(rename = "phoneNumber")]
        phone_number: String,
        otp: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct VerificationAttemptRequest {
    pub method: VerificationSubmission,
}

async fn two_factor_verification_attempt(
    auth: AuthClaims,
    cookies: Cookies,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<VerificationAttemptRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    // AuthN: Allow weak validations here because the user has to exist, but if this is their first
    // phone number/email 2fa then they won't be able to be strongly authenticated.
    let user = authenticate_weak(&auth, &dao).await?;

    let canonical = match &req.method {
        VerificationSubmission::Phone { phone_number, .. } => {
            canonicalize_phone_number(&phone_number).map_err(|_| {
                AntOnTheWebError::Validation(ValidationError::one(ValidationMessage::invalid(
                    "submission.phoneNumber",
                )))
            })?
        }
        VerificationSubmission::Email { email, .. } => {
            canonicalize_email(&email).map_err(|_| {
                AntOnTheWebError::Validation(ValidationError::one(ValidationMessage::invalid(
                    "submission.email",
                )))
            })?
        }
    };

    let method = match &req.method {
        VerificationSubmission::Email { .. } => {
            two_factor::VerificationMethod::Email(canonical.clone())
        }
        VerificationSubmission::Phone { .. } => {
            two_factor::VerificationMethod::Phone(canonical.clone())
        }
    };

    let user = authenticate_or_weak_matching_method(&auth, &dao, &method, user).await?;

    let verification = match &req.method {
        VerificationSubmission::Phone { otp, .. } => {
            two_factor::receive_phone_verification_code(&dao, &canonical, &otp).await?
        }
        VerificationSubmission::Email { otp, .. } => {
            two_factor::receive_email_verification_code(&dao, &canonical, &otp).await?
        }
    };

    match verification {
        VerificationReceipt::Failed => {
            return Ok((StatusCode::BAD_REQUEST, Json(false)).into_response())
        }

        VerificationReceipt::Success { user_id: _ } => {
            info!("Verification succeeded, user authenticated");

            // Add that contact method to the user since it's now verified
            match &req.method {
                VerificationSubmission::Phone { .. } => {
                    if !user.phone_numbers.contains(&canonical) {
                        info!(
                            "Adding phone number {} to user {}",
                            &canonical, &user.username
                        );
                        dao.users
                            .write()
                            .await
                            .add_phone_number_to_user(&user.user_id, &canonical)
                            .await?;
                    } else {
                        info!(
                            "Phone number {} already added to user {}",
                            &canonical, &user.username
                        );
                    }
                }
                VerificationSubmission::Email { .. } => {
                    if !user.emails.contains(&canonical) {
                        info!("Adding email {} to user {}", &canonical, &user.username);
                        dao.users
                            .write()
                            .await
                            .add_email_to_user(&user.user_id, &canonical)
                            .await?;
                    } else {
                        info!(
                            "Email {} already added to user {}",
                            &canonical, &user.username
                        );
                    }
                }
            }

            // Overwrite the old cookie with a new 2fa cookie
            cookies.add(make_auth_cookie(user.user_id.clone())?.1);

            return Ok((StatusCode::OK, "Verification successful.").into_response());
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AddPhoneNumberRequest {
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,

    #[serde(rename = "forceSend")]
    pub force_send: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "resolution")]
pub enum AddResolution {
    #[serde(rename = "added")]
    Added,
    #[serde(rename = "alreadyAdded")]
    AlreadyAdded,
}

#[derive(Serialize, Deserialize)]
pub struct AddPhoneNumberResponse {
    pub resolution: AddResolution,
}

async fn add_phone_number(
    auth: AuthClaims,
    State(InnerApiState { dao, sms, rng, .. }): ApiState,
    Json(req): Json<AddPhoneNumberRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    // AuthN: Allow weak authentication here because during signup process they need to be able
    // to add a phone number to get it validated. However, we only allow the user to be weakly
    // authenticated if they don't have any phone numbers or emails associated.
    let user = authenticate_weak(&auth, &dao).await?;

    info!("User adding phone number");
    let canonical_phone_number = {
        let validations = {
            let mut validations: Vec<ValidationMessage> = vec![];

            let canonical_phone_number = match canonicalize_phone_number(req.phone_number.as_str())
            {
                Ok(phone) => Ok(phone),
                Err(e) => {
                    info!("phone number {} was invalid: {}", req.phone_number, e);
                    validations.push(ValidationMessage::invalid("phoneNumber"));
                    Err(())
                }
            };

            match validations.as_slice() {
                &[] => Ok(canonical_phone_number.unwrap()),
                _ => Err(validations),
            }
        };

        validations.map_err(|v| AntOnTheWebError::Validation(ValidationError::many(v)))?
    };

    let method = two_factor::VerificationMethod::Phone(canonical_phone_number.clone());
    let user = authenticate_or_weak_matching_method(&auth, &dao, &method, user).await?;

    let already_added = {
        let by_phone_number = dao
            .users
            .read()
            .await
            .get_one_by_phone_number(&canonical_phone_number)
            .await?;

        let already_added = match by_phone_number {
            None => false,
            Some(other) if other.user_id == user.user_id => true,
            Some(_) => {
                return Ok((StatusCode::CONFLICT, "Phone number already exists.").into_response())
            }
        };

        already_added
    };

    // Send the SMS containing the one-time password
    if !already_added || (already_added && req.force_send) {
        let mut rng: tokio::sync::MutexGuard<'_, rand::prelude::StdRng> = rng.lock().await;

        two_factor::resend_phone_verification_code(
            &dao,
            sms.as_ref(),
            &mut rng,
            &user.user_id,
            &canonical_phone_number,
        )
        .await?;
    }

    if already_added {
        Ok((
            StatusCode::OK,
            Json(AddPhoneNumberResponse {
                resolution: AddResolution::AlreadyAdded,
            }),
        )
            .into_response())
    } else {
        Ok((
            StatusCode::OK,
            Json(AddPhoneNumberResponse {
                resolution: AddResolution::Added,
            }),
        )
            .into_response())
    }
}

#[derive(Serialize, Deserialize)]
pub struct AddEmailRequest {
    pub email: String,
    #[serde(rename = "forceSend")]
    pub force_send: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AddEmailResponse {
    pub resolution: AddResolution,
}

async fn add_email(
    auth: AuthClaims,
    State(InnerApiState {
        dao, rng, email, ..
    }): ApiState,
    Json(req): Json<AddEmailRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    // AuthN: Allow weak authentication here because during signup process they need to be able
    // to add an email to get it validated.
    let user = authenticate_weak(&auth, &dao).await?;

    info!("User adding email");

    let canonical_email = {
        let validations = {
            let mut validations: Vec<ValidationMessage> = vec![];

            let canonical_email = match canonicalize_email(&req.email) {
                Ok(phone) => Ok(phone),
                Err(e) => {
                    info!("email {} was invalid: {}", req.email, e);
                    validations.push(ValidationMessage::invalid("email"));
                    Err(())
                }
            };

            match validations.as_slice() {
                &[] => Ok(canonical_email.unwrap()),
                _ => Err(validations),
            }
        };

        validations.map_err(|v| AntOnTheWebError::Validation(ValidationError::many(v)))?
    };

    let method = two_factor::VerificationMethod::Email(canonical_email.clone());
    let user = authenticate_or_weak_matching_method(&auth, &dao, &method, user).await?;

    let already_added = {
        let by_email = dao
            .users
            .read()
            .await
            .get_one_by_email(&canonical_email)
            .await?;

        match by_email {
            None => false,
            Some(other) if other.user_id == user.user_id => true,
            Some(_) => return Ok((StatusCode::CONFLICT, "Email already exists.").into_response()),
        }
    };

    if !already_added || (already_added && req.force_send) {
        info!("Sending email verification code");
        let mut rng = rng.lock().await;
        two_factor::resend_email_verification_code(
            &dao,
            email.as_ref(),
            &mut rng,
            &user.user_id,
            &canonical_email,
        )
        .await?
    }

    if already_added {
        Ok((
            StatusCode::OK,
            Json(AddEmailResponse {
                resolution: AddResolution::AlreadyAdded,
            }),
        )
            .into_response())
    } else {
        Ok((
            StatusCode::OK,
            Json(AddEmailResponse {
                resolution: AddResolution::Added,
            }),
        )
            .into_response())
    }
}

#[derive(Serialize, Deserialize)]
pub struct PasswordResetCodeRequest {
    pub username: String,

    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
}

/// The first step in the password reset process, the user submits their information and receives
/// a one-time code if they exist.
async fn password_reset_code(
    State(InnerApiState { dao, rng, sms, .. }): ApiState,
    Json(req): Json<PasswordResetCodeRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let phone_number = canonicalize_phone_number(&req.phone_number).map_err(|_| {
        AntOnTheWebError::Validation(ValidationError::one(ValidationMessage::invalid(
            "phoneNumber",
        )))
    })?;

    let users = dao.users.read().await;

    let user = users.get_one_by_user_name(&req.username).await?;
    match user {
        Some(u) if u.phone_numbers.contains(&phone_number) => {
            let mut rng: tokio::sync::MutexGuard<'_, rand::prelude::StdRng> = rng.lock().await;

            match two_factor::resend_phone_verification_code(
                &dao,
                sms.as_ref(),
                &mut rng,
                &u.user_id,
                &phone_number,
            )
            .await
            {
                Ok(_) => Ok(()),
                Err(AntOnTheWebError::InternalServerError(s)) => {
                    Err(AntOnTheWebError::InternalServerError(s))
                }
                Err(e) => {
                    debug!(
                        "Swallowing validation error in password reset API to prevent leaks: {e:?}"
                    );
                    Ok(())
                }
            }?;
        }
        Some(_) => {
            info!(
                "user '{}' did not have phone number '{}'",
                req.username, phone_number
            );
        }
        None => {
            info!("no user '{}'", req.username);
        }
    }

    // Important, return the same response no matter if the user is wrong or right.
    return Ok((StatusCode::OK, "One-time code sent.").into_response());
}

#[derive(Serialize, Deserialize)]
pub struct PasswordResetSecretRequest {
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,

    pub otp: String,
}

#[derive(Serialize, Deserialize)]
pub struct PasswordResetSecretResponse {
    pub secret: String,
}

/// The second step in the password reset process. The user gives a one-time code and the server
/// returns a secret that they can later use alongside their new password to verify.
async fn password_reset_secret(
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<PasswordResetSecretRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let phone_number = canonicalize_phone_number(&req.phone_number)?;

    match two_factor::receive_phone_verification_code(&dao, &phone_number, &req.otp).await? {
        VerificationReceipt::Failed => {
            return Ok((StatusCode::BAD_REQUEST, "Invalid code.").into_response());
        }

        VerificationReceipt::Success { user_id } => {
            let secret_claims = PasswordResetClaims::new(user_id, phone_number);
            let jwt = encode_jwt(&secret_claims)?;

            return Ok((
                StatusCode::OK,
                Json(PasswordResetSecretResponse { secret: jwt }),
            )
                .into_response());
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
struct PasswordResetClaims {
    /// jwt subject, from the standard
    pub sub: UserId,

    /// jwt expiration (unix seconds timestamp) from the standard
    /// Must be u64 or won't be decodable:
    /// https://docs.rs/jsonwebtoken/latest/src/jsonwebtoken/validation.rs.html#188
    exp: u64,

    /// EXTRA
    /// the phone number they performed the otp process with
    pub phone_number: String,
}

impl PasswordResetClaims {
    pub fn new(user_id: UserId, phone_number: String) -> Self {
        Self {
            sub: user_id,
            exp: (Utc::now() + Duration::minutes(15)).timestamp() as u64,
            phone_number,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PasswordRequest {
    pub secret: String,
    pub password1: String,
    pub password2: String,
}

/// The third and final step in the password reset process. The user returns the secret and their
/// new password, and the server overwrites that user's password.
///
/// The auth claims are optional because if they aren't included, it's a "forgot my password"
/// but if they are, it's just changing your password.
async fn password(
    auth: Option<AuthClaims>,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<PasswordRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    {
        let mut validations: Vec<ValidationMessage> = vec![];
        if req.password1 != req.password2 {
            validations.push(ValidationMessage::new("password", "Passwords must match"));
        }

        validations.append(&mut validate_password(&req.password1));

        if validations.len() > 0 {
            return Err(AntOnTheWebError::Validation(ValidationError::many(
                validations,
            )));
        }
    }

    match auth {
        Some(auth) => {
            let user = authenticate(&auth, &dao).await?;

            dao.users
                .write()
                .await
                .overwrite_user_password(&user.user_id, &req.password1)
                .await?;

            return Ok((StatusCode::OK, "Password changed.").into_response());
        }

        None => {
            if req.password1 != req.password2 {
                return Err(AntOnTheWebError::Validation(ValidationError::one(
                    ValidationMessage::new("password", "Passwords must match"),
                )));
            }

            let claims = decode_jwt::<PasswordResetClaims>(&req.secret)?;
            let user_id = claims.sub;

            dao.users
                .write()
                .await
                .overwrite_user_password(&user_id, &req.password1)
                .await?;

            return Ok((StatusCode::OK, "Password changed.").into_response());
        }
    }
}

fn validate_username(username: &str) -> Vec<ValidationMessage> {
    let mut validations = vec![];

    let username_len = username.len();
    if username_len < 3 || username_len > 16 {
        validations.push(ValidationMessage::new(
            "username",
            "Field must be between 3 and 16 characters.",
        ));
    }

    let username_regex = regex::Regex::new(r"^[a-z0-9]{3,16}$").expect("invalid username regex");
    if !username_regex.is_match(&username) {
        validations.push(ValidationMessage::new(
            "username",
            "Field must contain only lowercase characters (a-z) and numbers (0-9).",
        ));
    }

    return validations;
}

fn validate_password(password: &str) -> Vec<ValidationMessage> {
    let mut validations: Vec<ValidationMessage> = vec![];

    let password_len = password.len();
    if password_len < 8 || password_len > 64 {
        validations.push(ValidationMessage::new(
            "password",
            "Field must be between 8 and 64 characters.",
        ));
    }

    if !password.contains("ant") {
        validations.push(ValidationMessage::new(
                 "password",
                                 "Field must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly." 
            ));
    }

    return validations;
}

#[derive(Serialize, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub password2: String,
}

async fn signup_request(
    cookies: Cookies,
    State(InnerApiState { dao, .. }): ApiState,
    Json(signup_request): Json<SignupRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    info!("Validating signup request...");

    let validations = {
        let mut validations: Vec<ValidationMessage> = vec![];

        validations.append(&mut validate_username(&signup_request.username));

        if signup_request.password != signup_request.password2 {
            validations.push(ValidationMessage::new("password", "Passwords must match."));
        }

        validations.append(&mut validate_password(&signup_request.password));

        match validations.as_slice() {
            &[] => Ok(()),
            _ => Err(validations),
        }
    };

    let _ = validations.map_err(|v| AntOnTheWebError::Validation(ValidationError::many(v)))?;

    {
        info!("Checking if user already exists...");
        let read_users = dao.users.read().await;

        let by_username = read_users
            .get_one_by_user_name(&signup_request.username)
            .await?
            .is_some();

        if by_username {
            return Err(AntOnTheWebError::ConflictError("User already exists."));
        }
    }

    let user = {
        // Make user
        info!("User does not exist, creating...");
        let mut write_users = dao.users.write().await;
        let user = write_users
            .create_user(
                signup_request.username,
                signup_request.password,
                "user".to_string(),
            )
            .await?;
        info!("Created user {}", user.user_id);

        user
    };

    // Make a weak auth token for the user for 2fa routes
    cookies.add(make_weak_auth_cookie(user.user_id.clone())?.1);

    return Ok((StatusCode::OK, "Signup completed.").into_response());
}

pub fn router() -> ApiRouter {
    Router::new()
        .route_with_tsr("/subscribe-newsletter", post(subscribe_email))
        .route_with_tsr("/phone-number", post(add_phone_number))
        .route_with_tsr("/email", post(add_email))
        .route_with_tsr("/password-reset-code", post(password_reset_code))
        .route_with_tsr("/password-reset-secret", post(password_reset_secret))
        .route_with_tsr("/password", post(password))
        .route_with_tsr("/login", post(login))
        .route_with_tsr("/logout", post(logout))
        .route_with_tsr("/signup", post(signup_request))
        .route_with_tsr(
            "/verification-attempt",
            post(two_factor_verification_attempt),
        )
        .route_with_tsr("/user", get(get_user_by_name))
        .route_with_tsr("/username", post(change_username))
        .route_with_tsr("/user/{user_name}", get(get_user_by_name))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /subscribe-newsletter",
                "POST /phone-number",
                "POST /email",
                "POST /password-reset-code",
                "POST /password-reset-secret",
                "POST /password",
                "POST /login",
                "POST /logout",
                "POST /signup",
                "POST /verification-attempt",
                "GET /user",
                "GET /user/{user_name}",
            ])
        })
}
