use std::fmt;
use std::sync::Arc;
use std::sync::LazyLock;

use ant_data_farm::users::User;
use ant_data_farm::AntDataFarmClient;
use ant_data_farm::{users::UserId, DaoTrait};
use ant_library::get_mode;
use ant_library::Mode;
use axum::extract::FromRequestParts;
use axum::extract::OptionalFromRequestParts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::RequestPartsExt;
use axum_extra::{
    extract::cookie::{Cookie, SameSite},
    headers, TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use tracing::error;
use tracing::info;
use tracing::warn;

pub static AUTH_KEYS: LazyLock<AuthKeys> = LazyLock::new(|| {
    let secret = dotenv::var("ANT_ON_THE_WEB_JWT_SECRET").expect("jwt secret");
    AuthKeys::new(secret.as_bytes())
});

pub struct AuthKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl AuthKeys {
    pub fn new(secret: &[u8]) -> Self {
        AuthKeys {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaims {
    /// JWT subject, as per standard.
    pub sub: UserId,
    /// JWT expiration, as per standard.
    exp: usize,
}

impl AuthClaims {
    pub fn new(user_id: UserId) -> Self {
        AuthClaims {
            sub: user_id,
            exp: 2000000000, // may 2033, my problem then!
        }
    }
}

/// Implement OptionalFromRequestParts for AuthClaims because not every API
/// needs to be strictly authenticated, it just helps if it is. For example, /api/ants/suggest
/// does not require it, but should use it if it's included.
impl<S> OptionalFromRequestParts<S> for AuthClaims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        // Extract the Bearer token header
        let cookie = match parts.extract::<TypedHeader<headers::Cookie>>().await.ok() {
            Some(TypedHeader(cookie)) if cookie.len() != 0 => cookie,
            None | Some(_) => {
                info!("cookie not included for optional auth, skipping...");
                return Ok(None);
            }
        };

        // If the user specifies a cookie, it has to have the right properties.
        let jwt = match cookie.get("typesofants_auth") {
            Some(cookie) => cookie,
            None => {
                warn!("Cookie {:?} had no 'typesofants_auth' key", cookie);
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

        return Ok(Some(claim_data.claims));
    }
}

/// Implement FromRequestParts for AuthClaims for APIs that absolutely need to be authenticated,
/// for example, /api/users/user, /api/users/logout, or other profile information.
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

#[derive(Debug)]
pub enum AuthError {
    AccessDenied(Option<String>),
    InternalServerError(anyhow::Error),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthError::AccessDenied(_) => write!(f, "AuthError::AccessDenied"),
            AuthError::InternalServerError(_) => write!(f, "AuthError::InternalServerError"),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::AccessDenied(identity) => {
                warn!("Access denied to identity: {:?}", identity);
                (StatusCode::UNAUTHORIZED, "Access denied.").into_response()
            }
            AuthError::InternalServerError(e) => {
                error!("AuthError::InternalServerError {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.",
                )
                    .into_response()
            }
        }
    }
}

impl<E> From<E> for AuthError
where
    E: std::fmt::Display + Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self::InternalServerError(value.into())
    }
}

pub async fn authenticate(
    auth: &AuthClaims,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    let users = dao.users.read().await;
    let user = users.get_one_by_id(&auth.sub).await?;
    match user {
        None => Err(AuthError::AccessDenied(Some(auth.sub.to_string()))),
        Some(u) => Ok(u),
    }
}

pub fn make_cookie(jwt: String) -> Cookie<'static> {
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
