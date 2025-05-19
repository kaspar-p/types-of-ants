use std::str::FromStr;
use std::sync::Arc;
use std::sync::LazyLock;

use crate::types::{DbRouter, DbState};
use ant_data_farm::users::verify_password_hash;
use ant_data_farm::users::User;
use ant_data_farm::users::UserId;
use ant_data_farm::AntDataFarmClient;
use ant_data_farm::DaoTrait;
use ant_library::get_mode;
use ant_library::Mode;
use axum::extract::FromRequestParts;
use axum::RequestPartsExt;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::SameSite;
use axum_extra::headers;
use axum_extra::{extract::cookie::Cookie, routing::RouterExt, TypedHeader};
use http::header;
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

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

#[derive(Debug, Deserialize, Serialize)]
pub struct GetUserResponse {
    pub user: User,
}
async fn get_user_by_name(
    auth: AuthClaims,
    path: Option<Path<String>>,
    State(dao): DbState,
) -> Result<impl IntoResponse, UsersError> {
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
        return Err(UsersError::AccessDenied(Some(user.user_id.to_string())));
    }
    info!("Granted access to {}", user.user_id);

    let users = dao.users.read().await;
    let user = users.get_one_by_user_name(&user_name).await?.unwrap();
    return Ok((
        StatusCode::OK,
        Json(GetUserResponse { user }).into_response(),
    ));
}

async fn authenticate(auth: &AuthClaims, dao: &Arc<AntDataFarmClient>) -> Result<User, UsersError> {
    let users = dao.users.read().await;
    let user = users.get_one_by_id(&auth.sub).await?;
    match user {
        None => Err(UsersError::AccessDenied(Some(auth.sub.to_string()))),
        Some(u) => Ok(u),
    }
}

async fn logout(auth: AuthClaims, State(dao): DbState) -> Result<impl IntoResponse, UsersError> {
    authenticate(&auth, &dao).await?;

    let mut cookie_expiration = make_cookie("".to_string());
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
    State(dao): DbState,
    Json(login_request): Json<LoginRequest>,
) -> Result<impl IntoResponse, UsersError> {
    let users = dao.users.read().await;
    let user: Option<User> = match &login_request.method {
        LoginMethod::Email(email) => match canonicalize_email(email.as_str()) {
            Err(e) => {
                info!("Field method.email invalid: {}", e);
                return Ok((StatusCode::BAD_REQUEST, "Field method.email invalid.").into_response());
            }
            Ok(email) => users.get_one_by_email(&email).await?,
        },
        LoginMethod::Phone(phone) => match canonicalize_phone_number(phone.as_str()) {
            Err(e) => {
                info!("Field method.phone invalid: {}", e);
                return Ok((StatusCode::BAD_REQUEST, "Field method.phone invalid.").into_response());
            }
            Ok(phone) => users.get_one_by_phone_number(&phone).await?,
        },
        LoginMethod::Username(username) => users.get_one_by_user_name(&username).await?,
    };

    let user = match user {
        None => {
            return Ok((StatusCode::UNAUTHORIZED, "Access denied.").into_response());
        }
        Some(user) => user,
    };

    if !verify_password_hash(
        login_request.password.as_str(),
        &user.password_hash.as_str(),
    )? {
        return Ok((StatusCode::UNAUTHORIZED, "Access denied.").into_response());
    }

    let claims = AuthClaims {
        sub: user.user_id.clone(),
        exp: 2000000000, // may 2033, my problem then!
    };

    let jwt = jsonwebtoken::encode(&Header::default(), &claims, &AUTH_KEYS.encoding)?;

    // TODO: Verify with two-factor

    let cookie = make_cookie(jwt.clone());

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

fn make_cookie(jwt: String) -> Cookie<'static> {
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie
    Cookie::build(("typesofants_auth", jwt.clone()))
        .secure(true)
        .http_only(true)
        .permanent()
        .path("/")
        .same_site(match get_mode() {
            Mode::Dev => SameSite::None,
            Mode::Prod => SameSite::Strict,
        })
        .build()
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthClaims {
    /// JWT subject, as per standard.
    sub: UserId,
    /// JWT expiration, as per standard.
    exp: usize,
}

impl<S> FromRequestParts<S> for AuthClaims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract the Bearer token header
        let TypedHeader(cookie) = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(|e| {
                warn!("Invalid authorization token: {e}");
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid authorization token.".to_string(),
                );
            })?;

        let jwt = match cookie.get("typesofants_auth") {
            Some(cookie) => cookie,
            None => {
                warn!("No 'typesofants_auth' cookie found.");
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid authorization token.".to_string(),
                ));
            }
        };

        // Decode claim data
        let claim_data = decode::<AuthClaims>(jwt, &AUTH_KEYS.decoding, &Validation::default())
            .map_err(|e| {
                warn!("Unauthorized access attempted: {e}");
                return (StatusCode::UNAUTHORIZED, "Access denied.".to_string());
            })?;

        return Ok(claim_data.claims);
    }
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
                    return Err(UsersError::ValidationError(ValidationMessage {
                        field: "phoneNumber".to_string(),
                        msg: "Field invalid.".to_string(),
                    }));
                }
                Ok(phone_number) => phone_number,
            };

        let canonical_email = match canonicalize_email(&signup_request.email.as_str()) {
            Err(e) => {
                info!("Signup request email {}: {}", signup_request.email, e);
                return Err(UsersError::ValidationError(ValidationMessage {
                    field: "email".to_string(),
                    msg: "Field invalid.".to_string(),
                }));
            }
            Ok(email) => email,
        };

        let username_len = signup_request.username.len();
        if username_len < 3 || username_len > 16 {
            return Err(UsersError::ValidationError(ValidationMessage {
                field: "username".to_string(),
                msg: "Field must be between 3 and 16 characters.".to_string(),
            }));
        }

        let username_regex =
            regex::Regex::new(r"^[a-z0-9]{3,16}$").expect("invalid username regex");
        if !username_regex.is_match(&signup_request.username) {
            return Err(UsersError::ValidationError(ValidationMessage {
                field: "username".to_string(),
                msg: "Field must contain only lowercase characters (a-z) and numbers (0-9)."
                    .to_string(),
            }));
        }

        let password_len = signup_request.password.len();
        if password_len < 8 || password_len > 64 {
            return Err(UsersError::ValidationError(ValidationMessage {
                field: "password".to_string(),
                msg: "Field must be between 8 and 64 characters.".to_string(),
            }));
        }

        if !signup_request.password.contains("ant") {
            return Err(UsersError::ValidationError(ValidationMessage {
                field: "password".to_string(),
                msg: "Field must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly.".to_string() 
                    }));
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
            return Err(UsersError::ConflictError(UserTaken {
                msg: "User already exists.".to_string(),
            }));
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
        info!("Created user {}", user.user_id);
    }

    return Ok((StatusCode::OK, "Signup completed.").into_response());
}

pub fn router() -> DbRouter {
    Router::new()
        .route_with_tsr("/subscribe-newsletter", post(add_anonymous_email))
        .route_with_tsr("/login", post(login))
        .route_with_tsr("/logout", post(logout))
        .route_with_tsr("/signup", post(signup_request))
        // .route_with_tsr("/verification", post(verification))
        .route_with_tsr("/user", get(get_user_by_name))
        .route_with_tsr("/user/{user_name}", get(get_user_by_name))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /subscribe-newsletter",
                "POST /login",
                "POST /logout",
                "POST /signup",
                "GET /user",
                "GET /user/{user_name}",
            ])
        })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub field: String,
    pub msg: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserTaken {
    pub msg: String,
}

enum UsersError {
    AccessDenied(Option<String>),
    InternalServerError(anyhow::Error),
    ValidationError(ValidationMessage),
    ConflictError(UserTaken),
}

impl IntoResponse for UsersError {
    fn into_response(self) -> Response {
        match self {
            UsersError::InternalServerError(e) => {
                error!("UsersError::InternalServerError {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.",
                )
            }
            .into_response(),

            UsersError::AccessDenied(identity) => {
                warn!("Access denied to identity: {:?}", identity);
                (StatusCode::UNAUTHORIZED, "Access denied.").into_response()
            }

            UsersError::ValidationError(msg) => {
                warn!("UsersError::ValidationError {:?}", msg);
                (StatusCode::BAD_REQUEST, Json(msg))
            }
            .into_response(),

            UsersError::ConflictError(taken) => {
                warn!("UsersError::ConflictError {:?}", taken);
                (StatusCode::CONFLICT, Json(taken))
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
