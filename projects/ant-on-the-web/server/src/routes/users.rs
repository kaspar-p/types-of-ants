use std::str::FromStr;
use std::sync::LazyLock;

use crate::types::{DbRouter, DbState};
use ant_data_farm::users::verify_password_hash;
use ant_data_farm::users::User;
use ant_data_farm::users::UserId;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use serde::{Deserialize, Serialize};
use tracing::error;
use tracing::{debug, info};

static AUTH_KEYS: LazyLock<AuthKeys> = LazyLock::new(|| {
    let secret = dotenv::var("ANT_ON_THE_WEB_JWT_SECRET").expect("jwt secret");
    AuthKeys::new(secret.as_bytes())
});

struct AuthKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl AuthKeys {
    pub fn new(secret: &[u8]) -> Self {
        AuthKeys {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Deserialize)]
pub struct EmailRequest {
    email: String,
}
async fn add_anonymous_email(
    State(dao): DbState,
    Json(EmailRequest { email }): Json<EmailRequest>,
) -> Result<impl IntoResponse, UsersError> {
    debug!("Subscribing with email {}", email);

    let exists = {
        let users = dao.users.read().await;
        users
            .get_one_by_user_name("nobody")
            .await?
            .map(|u| u.emails.contains(&email))
    };
    match exists {
        None => {
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to validate uniqueness").into_response(),
            ))
        }
        Some(email_exists) => {
            if email_exists {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    Json(format!("Email '{email}' is already subscribed!")).into_response(),
                ));
            }
        }
    }

    let nobody_user: Option<User> = {
        let users = dao.users.read().await;
        users.get_one_by_user_name("nobody").await?
    };
    if nobody_user.is_none() {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("Failed attaching email!").into_response(),
        ));
    }

    let mut user_write = dao.users.write().await;
    user_write
        .add_email_to_user(nobody_user.unwrap().user_id, email)
        .await?;

    return Ok((
        StatusCode::OK,
        Json("Subscribed!".to_owned()).into_response(),
    ));
}

async fn get_user_by_name(
    Path(user_name): Path<String>,
    State(dao): DbState,
) -> Result<impl IntoResponse, UsersError> {
    let users = dao.users.read().await;
    let user = users.get_one_by_user_name(&user_name).await?;
    match user {
        None => Ok((
            StatusCode::NOT_FOUND,
            Json(format!(
                "There was no user with username: {} found!",
                user_name
            )),
        )
            .into_response()),
        Some(user) => Ok((StatusCode::OK, Json(user).into_response()).into_response()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginMethod {
    #[serde(rename = "username")]
    Username(String),
    #[serde(rename = "email")]
    Email(String),
    #[serde(rename = "phone")]
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

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}
async fn login(
    State(dao): DbState,
    Json(login_request): Json<LoginRequest>,
) -> Result<impl IntoResponse, UsersError> {
    let users = dao.users.read().await;
    let user: Option<User> = match &login_request.method {
        LoginMethod::Email(email) => match canonicalize_email(email.as_str()) {
            Err(e) => {
                info!("Field method.email invalid: {}", e);
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Field method.email invalid.".into_response(),
                ));
            }
            Ok(email) => users.get_one_by_email(&email).await?,
        },
        LoginMethod::Phone(phone) => match canonicalize_phone_number(phone.as_str()) {
            Err(e) => {
                info!("Field method.phone invalid: {}", e);
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Field method.phone invalid.".into_response(),
                ));
            }
            Ok(phone) => users.get_one_by_phone_number(&phone).await?,
        },
        LoginMethod::Username(username) => users.get_one_by_user_name(&username).await?,
    };

    let user = match user {
        None => {
            return Ok((StatusCode::UNAUTHORIZED, "Access denied.".into_response()));
        }
        Some(user) => user,
    };

    if !verify_password_hash(
        login_request.password.as_str(),
        &user.password_hash.as_str(),
    )? {
        return Ok((StatusCode::UNAUTHORIZED, "Access denied.".into_response()));
    }

    let claims = AuthClaims {
        sub: user.username.clone(),
        exp: 2000000000, // may 2033, my problem then!
    };

    let jwt = jsonwebtoken::encode(&Header::default(), &claims, &AUTH_KEYS.encoding)?;

    // TODO: Verify with two-factor

    Ok((
        StatusCode::OK,
        Json(LoginResponse { token: jwt }).into_response(),
    ))
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthClaims {
    /// JWT subject, as per standard.
    sub: String,
    /// JWT expiration, as per standard.
    exp: usize,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationRequest {
    pub user_id: UserId,
    pub submission: VerificationSubmission,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum VerificationSubmission {
    #[serde(rename = "username")]
    Username { otp: String },

    #[serde(rename = "email")]
    Email { otp: String },

    #[serde(rename = "phone")]
    Phone { otp: String },
}
async fn post_verification(
    State(db): DbState,
    Json(signup_verification_request): Json<VerificationRequest>,
) -> impl IntoResponse {
    return (StatusCode::NOT_IMPLEMENTED, "Unimplemented");
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
    State(dao): DbState,
    Json(signup_request): Json<SignupRequest>,
) -> Result<impl IntoResponse, UsersError> {
    info!("Validating signup request...");
    let (canonical_email, canonical_phone_number) = {
        let canonical_phone_number =
            match canonicalize_phone_number(signup_request.phone_number.as_str()) {
                Err(e) => {
                    info!(
                        "Signup request phone number {} was invalid: {}",
                        signup_request.phone_number, e
                    );
                    return Ok(
                        (StatusCode::BAD_REQUEST, "Field phone_number invalid.").into_response()
                    );
                }
                Ok(phone_number) => phone_number,
            };

        let canonical_email = match canonicalize_email(&signup_request.email.as_str()) {
            Err(e) => {
                info!("Signup request email {}: {}", signup_request.email, e);
                return Ok((StatusCode::BAD_REQUEST, "Field email invalid.").into_response());
            }
            Ok(email) => email,
        };

        let username_len = signup_request.username.len();
        if username_len < 3 || username_len > 16 {
            return Ok((
                StatusCode::BAD_REQUEST,
                "Field username must be between 3 and 16 characters.",
            )
                .into_response());
        }

        let username_regex =
            regex::Regex::new(r"^[a-z0-9]{3,16}$").expect("invalid username regex");
        if !username_regex.is_match(&signup_request.username) {
            return Ok((
                StatusCode::BAD_REQUEST,
                "Field username must contain only lowercase characters and numbers.",
            )
                .into_response());
        }

        let password_len = signup_request.password.len();
        if password_len < 8 || password_len > 64 {
            return Ok((
                StatusCode::BAD_REQUEST,
                "Field password must be between 8 and 64 characters.",
            )
                .into_response());
        }

        if !signup_request.password.contains("ant") {
            return Ok((StatusCode::BAD_REQUEST, "Field password must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly.").into_response());
        }

        (canonical_email, canonical_phone_number)
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
            return Ok((StatusCode::CONFLICT, "User already exists.").into_response());
        }
    }

    {
        // Make user
        info!("User does not exist, creating...");
        let mut write_users = dao.users.write().await;
        let user = write_users
            .create_user(
                signup_request.username,
                canonical_phone_number,
                canonical_email,
                signup_request.password,
            )
            .await?;
    }

    return Ok((StatusCode::OK, "Signup completed.").into_response());
}

pub fn router() -> DbRouter {
    Router::new()
        .route_with_tsr("/subscribe-newsletter", post(add_anonymous_email))
        .route_with_tsr("/signup", post(signup_request))
        // .route_with_tsr("/verification", post(verification))
        .route_with_tsr("/login", post(login))
        .route_with_tsr("/user/{user_name}", get(get_user_by_name))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /signup",
                "POST /user/{user_name}",
                "POST /subscribe-newsletter",
            ])
        })
}

enum UsersError {
    InternalServerError(anyhow::Error),
}

impl IntoResponse for UsersError {
    fn into_response(self) -> Response {
        match self {
            UsersError::InternalServerError(e) => {
                error!("UsersError: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.",
                )
            }
            .into_response(),
        }
    }
}

impl<E> From<E> for UsersError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(err.into())
    }
}
