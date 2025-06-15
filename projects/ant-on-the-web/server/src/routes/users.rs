use std::str::FromStr;

use crate::{
    err::ValidationError,
    routes::lib::{
        auth::{make_auth_cookie, make_weak_auth_cookie},
        err::ValidationMessage,
        two_factor,
    },
    state::{ApiRouter, ApiState, InnerApiState},
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
use http::header;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::lib::{
    auth::{authenticate, cookie_defaults, optional_authenticate, weakly_authenticate, AuthClaims},
    err::AntOnTheWebError,
    two_factor::{VerificationMethod, VerificationState},
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
            return Err(AntOnTheWebError::ValidationError(ValidationError {
                errors: vec![ValidationMessage::invalid("email")],
            }));
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

async fn logout(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    authenticate(&auth, &dao).await?;

    let mut cookie_expiration = cookie_defaults("".to_string()).build();
    cookie_expiration.make_removal();

    return Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie_expiration.to_string())],
        "Logout successful.",
    ));
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginMethod {
    #[serde(rename = "username")]
    Username(String),
    #[serde(rename = "email")]
    Email(String),
    #[serde(rename = "phoneNumber")]
    Phone(String),
}

#[derive(Debug, Serialize, Deserialize)]
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
    State(InnerApiState { dao, .. }): ApiState,
    Json(login_request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let users = dao.users.read().await;
    let user: Result<Option<User>, ValidationMessage> = match &login_request.method {
        LoginMethod::Email(email) => match canonicalize_email(email.as_str()) {
            Err(e) => {
                info!("Field method.email invalid: {}", e);
                Err(ValidationMessage::invalid("method.email"))
            }
            Ok(email) => Ok(users.get_one_by_email(&email).await?),
        },
        LoginMethod::Phone(phone) => match canonicalize_phone_number(phone.as_str()) {
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
            return Err(AntOnTheWebError::ValidationError(ValidationError {
                errors: vec![v],
            }));
        }
        Ok(None) => {
            return Err(AntOnTheWebError::AccessDenied(None));
        }
        Ok(Some(user)) => user,
    };

    if !verify_password_hash(
        login_request.password.as_str(),
        &user.password_hash.as_str(),
    )? {
        return Err(AntOnTheWebError::AccessDenied(None));
    }

    // TODO: two-factor verify on login (to a method they have verified)

    debug!("Password verified, generating jwt token...");
    let (jwt, cookie) = make_weak_auth_cookie(user.user_id.clone())?;

    return Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie.to_string())],
        Json(LoginResponse {
            user_id: user.user_id.clone(),
            access_token: jwt,
        }),
    )
        .into_response());
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum VerificationSubmission {
    #[serde(rename = "email")]
    Email { otp: String },

    #[serde(rename = "phone")]
    Phone { otp: String },
}

#[derive(Serialize, Deserialize)]
pub struct VerificationRequest {
    pub method: VerificationMethod,
}

async fn create_two_factor_verification(
    auth: AuthClaims,
    State(InnerApiState { dao, sms, rng }): ApiState,
    Json(verification_request): Json<VerificationRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    // AuthN: Allow users who are not fully authenticated because they can choose to
    // to resend their validation codes during their initial validation
    let user = weakly_authenticate(&auth, &dao).await?;

    match verification_request.method {
        VerificationMethod::Phone(_) => {
            {
                let mut rng = rng.lock().await;
                two_factor::resend_phone_verification_code(&dao, sms.as_ref(), &mut rng, &user)
                    .await?;
            }

            return Ok((StatusCode::OK, "One-time code resent.").into_response());
        }
        VerificationMethod::Email(_) => {
            return Ok((StatusCode::NOT_IMPLEMENTED, "unimplemented").into_response());
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct VerificationAttemptRequest {
    pub submission: VerificationSubmission,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationAttemptResponse {
    pub token: String,
}

async fn two_factor_verification_attempt(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
    Json(signup_verification_request): Json<VerificationAttemptRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = weakly_authenticate(&auth, &dao).await?;

    let verification = match signup_verification_request.submission {
        VerificationSubmission::Phone { otp } => {
            two_factor::receive_phone_verification_code(&dao, &user, &otp).await?
        }
        VerificationSubmission::Email { otp: _ } => {
            return Ok((StatusCode::NOT_IMPLEMENTED, "unimplemented").into_response());
        }
    };

    match verification {
        VerificationState::NoVerificationFound
        | VerificationState::HasMoreAttempts
        | VerificationState::OutOfAttempts => {
            return Ok((StatusCode::BAD_REQUEST, Json(verification)).into_response())
        }

        VerificationState::Verified => {
            info!("Verification succeeded, user authenticated");

            let (jwt, cookie) = make_auth_cookie(user.user_id.clone())?;

            return Ok((
                StatusCode::OK,
                [(header::SET_COOKIE, cookie.to_string())], // Overwrite the old cookie with a new 2fa cookie
                Json(VerificationAttemptResponse { token: jwt.clone() }),
            )
                .into_response());
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub email: String,
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
    pub password: String,
}
async fn signup_request(
    State(InnerApiState { dao, sms, rng }): ApiState,
    Json(signup_request): Json<SignupRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    info!("Validating signup request...");

    let validations = {
        let mut validations: Vec<ValidationMessage> = vec![];

        let canonical_phone_number =
            match canonicalize_phone_number(signup_request.phone_number.as_str()) {
                Ok(phone) => Ok(phone),
                Err(e) => {
                    info!(
                        "Signup request phone number {} was invalid: {}",
                        signup_request.phone_number, e
                    );
                    validations.push(ValidationMessage::invalid("phoneNumber"));
                    Err(())
                }
            };

        let canonical_email = match canonicalize_email(&signup_request.email.as_str()) {
            Ok(email) => Ok(email),
            Err(e) => {
                info!("Signup request email {}: {}", signup_request.email, e);
                validations.push(ValidationMessage::invalid("email"));
                Err(())
            }
        };

        let username_len = signup_request.username.len();
        if username_len < 3 || username_len > 16 {
            validations.push(ValidationMessage::new(
                "username",
                "Field must be between 3 and 16 characters.",
            ));
        }

        let username_regex =
            regex::Regex::new(r"^[a-z0-9]{3,16}$").expect("invalid username regex");
        if !username_regex.is_match(&signup_request.username) {
            validations.push(ValidationMessage::new(
                "username",
                "Field must contain only lowercase characters (a-z) and numbers (0-9).",
            ));
        }

        let password_len = signup_request.password.len();
        if password_len < 8 || password_len > 64 {
            validations.push(ValidationMessage::new(
                "password",
                "Field must be between 8 and 64 characters.",
            ));
        }

        if !signup_request.password.contains("ant") {
            validations.push(ValidationMessage::new(
                 "password",
                                 "Field must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly." 
            ));
        }

        match validations.as_slice() {
            &[] => Ok((canonical_email.unwrap(), canonical_phone_number.unwrap())),
            _ => Err(validations),
        }
    };

    let (canonical_email, canonical_phone_number) = match validations {
        Err(v) => {
            return Err(AntOnTheWebError::ValidationError(ValidationError {
                errors: v,
            }));
        }
        Ok(data) => data,
    };

    {
        info!("Checking if user already exists...");
        let read_users = dao.users.read().await;

        let by_email = read_users
            .get_one_by_email(&canonical_email)
            .await?
            .is_some();
        let by_username = read_users
            .get_one_by_user_name(&signup_request.username)
            .await?
            .is_some();
        let by_phone = read_users
            .get_one_by_phone_number(&canonical_phone_number)
            .await?
            .is_some();
        if by_email || by_username || by_phone {
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
                canonical_phone_number,
                canonical_email,
                signup_request.password,
                "user".to_string(),
            )
            .await?;
        info!("Created user {}", user.user_id);

        user
    };

    // Start verifying their two-factor phone number
    // This process happens async, but they won't be able to login until it's verified.
    // The UI just has to accept the 200 OK and continue the user into some fields where they can
    // attempt to fullfil the OTP for each method.
    {
        let mut rng = rng.lock().await;
        two_factor::send_phone_verification_code(&dao, sms.as_ref(), &mut rng, &user).await?;
    }

    // TODO: Start verifying their two-factor email address
    {}

    // Make a weak auth token for the user for 2fa routes
    let (_, cookie) = make_weak_auth_cookie(user.user_id.clone())?;

    return Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie.to_string())],
        "Signup completed.",
    )
        .into_response());
}

pub fn router() -> ApiRouter {
    Router::new()
        .route_with_tsr("/subscribe-newsletter", post(subscribe_email))
        .route_with_tsr("/login", post(login))
        .route_with_tsr("/logout", post(logout))
        .route_with_tsr("/signup", post(signup_request))
        .route_with_tsr("/verification", post(create_two_factor_verification))
        .route_with_tsr(
            "/verification-attempt",
            post(two_factor_verification_attempt),
        )
        .route_with_tsr("/user", get(get_user_by_name))
        .route_with_tsr("/user/{user_name}", get(get_user_by_name))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /subscribe-newsletter",
                "POST /login",
                "POST /logout",
                "POST /signup",
                "POST /verification",
                "POST /verification-attempt",
                "GET /user",
                "GET /user/{user_name}",
            ])
        })
}
