use std::{fmt, sync::Arc};

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
use cookie::{
    time::{Duration, OffsetDateTime},
    CookieBuilder,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use super::{
    jwt,
    two_factor::{self, VerificationMethod},
};

/// The cookie is a secret artifact that (if correct), proves the user is who they say they are.
const AUTH_COOKIE_NAME: &str = "typesofants_auth";

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaims {
    /// JWT subject, as per standard.
    pub sub: UserId,

    /// JWT expiration, Unix seconds timestamp, from the standard.
    /// Must also be u64 because the Validation tries to parse as that:
    /// https://docs.rs/jsonwebtoken/latest/src/jsonwebtoken/validation.rs.html#188
    exp: u64,

    /// Whether the user still needs to perform 2fa
    /// This is stored (signed) as a cookie in the user's browser so that the /login
    /// route can authenticate the user for some user-specific routes, but they don't
    /// yet have full access.
    /// This field being `true` means the user has yet to perform 2fa, and they should
    /// only be able to access routes to resend 2fa codes, submit 2fa codes, and logout.
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
        let claim_data = jwt::decode_jwt::<AuthClaims>(jwt).map_err(|e| {
            warn!("decode jwt failed: {e:?}");
            return (StatusCode::UNAUTHORIZED, "Access denied.".to_string());
        })?;

        return Ok(Some(claim_data));
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
        let claim_data = jwt::decode_jwt::<AuthClaims>(jwt).map_err(|e| {
            warn!("decode jwt failed: {e:?}");
            return (StatusCode::UNAUTHORIZED, "Access denied.".to_string());
        })?;

        return Ok(claim_data);
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
///
/// The `method` is used to ensure that the weak authentication is only valid for
///
/// Unless this is a route that deals with auth, you should use [`authenticate`].
pub async fn weakly_authenticate(
    auth: &AuthClaims,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    let user = {
        let users = dao.users.read().await;
        let user = users.get_one_by_id(&auth.sub).await?;
        user
    };

    match user {
        None => Err(AuthError::AccessDenied(Some(auth.sub.to_string()))),
        Some(user) => Ok(user),
    }
}

/// Routes like POST /users/phone-number should either allow strongly auth'd users (for adding new phone numbers)
/// in all circumstances, or should allow weakly auth'd users if they are using a phone number/email that is
/// already registered under that user, or if they have no phone numbers/emails at all.
pub async fn authenticate_or_weak_matching_method(
    auth: &AuthClaims,
    dao: &AntDataFarmClient,
    attempt: &VerificationMethod,
    user: User,
) -> Result<User, AuthError> {
    // All strong auth is also weak auth, let them in.
    if !auth.needs_2fa {
        info!("Allow STRONG auth user '{}'.", user.username);
        return Ok(user);
    }

    let verifications = two_factor::user_is_two_factor_verified(&dao, &user).await?;
    if verifications.verified.is_empty() {
        info!("Allow WEAK auth for '{}', no verifications.", user.username);
        return Ok(user);
    }

    // Allow the user in because they have weak auth and they are doing 2fa with a registered attempt
    if verifications.verified.contains(&attempt) {
        info!(
            "Allow WEAK authentication user {}:{} with verifications {:?} because attempt {:?} matches",
            user.user_id, user.username, verifications, attempt
        );
        return Ok(user);
    }

    info!(
        "Rejecting user {}:{} with verifications {:?} and attempt {:?}",
        user.user_id, user.username, verifications, attempt
    );
    return Err(AuthError::AccessDenied(Some(
        "rejecting user for bad weak matching attempt".to_string(),
    )));
}

fn upgrade_weak_auth(auth: &AuthClaims, user: User) -> Result<User, AuthError> {
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

/// Require that the claims included in the request are valid. Use this for all authenticated routes.
/// Returns an AuthError if something went wrong, either a DB error or access denial.
/// This authentication state is the final one for the user, and requires they are 2FA verified.
pub async fn authenticate(
    auth: &AuthClaims,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    return upgrade_weak_auth(&auth, weakly_authenticate(&auth, &dao).await?);
}

/// If the auth claims are present, return the user that is authenticated. If not, returns the
/// 'nobody' anonymous user.
///
/// Returns AccessDenied errors if the user is specified but the claims are tampered or somehow
/// wrong.
///
/// This is used in public APIs, for private user-specific APIs default to using [`authenticate`].
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

/// This function should be used to _weakly_ authenticate the user. They might be signed in
/// but they have yet to complete the 2fa flow. They should only be able to access routes
/// for submitting 2fa codes, resending them, or logging out.
pub fn make_weak_auth_cookie(user_id: UserId) -> Result<(String, Cookie<'static>), anyhow::Error> {
    debug!("Making JWT weak cookie for the browser...");
    let claims = AuthClaims::new(user_id);
    let jwt = jwt::encode_jwt(&claims)?;

    let cookie = cookie_defaults(jwt.clone())
        .expires(OffsetDateTime::now_utc().saturating_add(Duration::minutes(15)))
        .build();

    Ok((jwt, cookie))
}

/// This function should be used to fully authenticate the user, after they have performed the 2fa flow
/// This cookie is a secret that should allow the user to hit every route they can, with no restrictions.
pub fn make_auth_cookie(user_id: UserId) -> Result<(String, Cookie<'static>), anyhow::Error> {
    let claims: AuthClaims = AuthClaims::two_factor_verified(user_id);
    let jwt = jwt::encode_jwt(&claims)?;

    debug!("Making JWT auth cookie for the browser...");
    let cookie = cookie_defaults(jwt.clone()).build();

    Ok((jwt, cookie))
}

pub fn cookie_defaults(jwt: String) -> CookieBuilder<'static> {
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
}
