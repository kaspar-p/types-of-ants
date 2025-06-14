use std::{
    fmt,
    sync::{Arc, LazyLock},
};

use ant_data_farm::{
    users::{User, UserId},
    AntDataFarmClient, DaoTrait,
};
use ant_library::{get_mode, Mode};
use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::StatusCode,
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use axum_extra::{
    extract::cookie::{Cookie, SameSite},
    headers, TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

pub static AUTH_KEYS: LazyLock<AuthKeys> = LazyLock::new(|| {
    debug!("Initializing jwt secret...");
    let secret = dotenv::var("ANT_ON_THE_WEB_JWT_SECRET").expect("jwt secret");

    debug!("jwt secret initialized...");
    AuthKeys::new(secret.as_bytes())
});

/// The cookie is a secret artifact that (if correct), proves the user is who they say they are.
const AUTH_COOKIE_NAME: &str = "typesofants_auth";

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
    exp: u64,
    /// Whether the user still needs to perform 2fa
    #[serde(default = "default_needs_2fa")] // backwards compat
    pub needs_2fa: bool,
}

fn default_needs_2fa() -> bool {
    true
}

impl AuthClaims {
    pub fn new(user_id: UserId) -> Self {
        AuthClaims {
            sub: user_id,
            exp: 2000000000, // may 2033, my problem then!
            needs_2fa: true,
        }
    }

    pub fn two_factor_verified(user_id: UserId) -> Self {
        AuthClaims {
            needs_2fa: false,
            ..Self::new(user_id)
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
        // Extract the Cookie header
        let cookie = match parts.extract::<TypedHeader<headers::Cookie>>().await.ok() {
            Some(TypedHeader(cookie)) if cookie.len() != 0 => cookie,
            None | Some(_) => {
                info!("cookie not included for optional auth, skipping...");
                return Ok(None);
            }
        };

        // If the user specifies a cookie, it has to have the right properties.
        let jwt = match cookie.get(AUTH_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => {
                warn!("Cookie {:?} had no '{}' key", cookie, AUTH_COOKIE_NAME);
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

        let jwt = match cookie.get(AUTH_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => {
                warn!("No '{}' cookie found.", AUTH_COOKIE_NAME);
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

/// This function allows any user in that has successfully signed up, but may not have performed
/// their two-factor verification. This should be used for routes required to start/finish the
/// two-factor verification flows.
pub async fn authenticate_for_two_factor(
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

/// Require that the claims included in the request are valid. Use this for all authenticated routes.
/// Returns an AuthError if something went wrong, either a DB error or access denial.
/// This authentication state is the final one for the user, and requires they are 2FA verified.
pub async fn authenticate(
    auth: &AuthClaims,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    let user = authenticate_for_two_factor(&auth, &dao).await?;

    // We let the user in only if they don't need 2fa. This comes from AuthClaims and is signed
    // by the server's key, so we can trust that the server wrote this during /login, but it
    // never got replaced during /verification
    if auth.needs_2fa {
        return Err(AuthError::AccessDenied(Some(
            "user is not two-factor verified".to_string(),
        )));
    } else {
        return Ok(user);
    }
}

/// If the auth claims are present, return the user that is authenticated. If not, returns the
/// 'nobody' anonymous user.
///
/// Returns AccessDenied errors if the user is specified but the claims are tampered or somehow
/// wrong.
pub async fn optional_authenticate(
    auth: Option<&AuthClaims>,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    match auth {
        None => {
            let users = dao.users.read().await;
            Ok(users
                .get_one_by_user_name("nobody")
                .await?
                .expect("nobody user exists"))
        }
        Some(auth) => authenticate(&auth, &dao).await,
    }
}

pub fn make_cookie(jwt: String) -> Cookie<'static> {
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie
    Cookie::build((AUTH_COOKIE_NAME, jwt.clone()))
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
