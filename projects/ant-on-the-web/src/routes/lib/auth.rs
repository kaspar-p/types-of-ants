use std::{fmt, sync::Arc};

use ant_data_farm::{
    users::{User, UserId},
    AntDataFarmClient, DaoTrait,
};
use ant_library::{get_mode, Mode};
use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use cookie::{
    time::{Duration, OffsetDateTime},
    CookieBuilder,
};
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use tracing::{debug, error, info, warn};

use crate::{err::AntOnTheWebError, state::InnerApiState};

use super::{
    jwt,
    two_factor::{self, VerificationMethod},
};

/// The cookie is a secret artifact that (if correct), proves the user is who they say they are.
pub const AUTH_COOKIE_NAME: &str = "typesofants_auth";

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

async fn try_api_token_authentication(
    state: &InnerApiState,
    cookie: &str,
) -> Result<AuthClaims, (StatusCode, String)> {
    let split: Vec<&str> = cookie.split(":").collect();
    if split.len() != 2 {
        warn!("API token not structured as user:token");
        return Err((StatusCode::UNAUTHORIZED, "Access denied.".to_string()));
    }

    let username: &str = split[0];
    let token: &str = split[1];

    let api_tokens = state.dao.api_tokens.read().await;

    let user = api_tokens
        .verify_token_user(username, token)
        .await
        .map_err(|e| {
            error!("Attempting API Token validation failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong, please retry.".to_string(),
            )
        })?;

    match user {
        None => {
            warn!("API token not valid.");
            Err((StatusCode::UNAUTHORIZED, "Access denied.".to_string()))
        }
        // Two-factor verification not required for API access!
        Some(u) => Ok(AuthClaims::two_factor_verified(u)),
    }
}

async fn decode_jwt_cookie(
    state: &InnerApiState,
    cookie: &str,
) -> Result<AuthClaims, (StatusCode, String)> {
    // If cookie is specified it can't be tampered with Decode claim data
    let claim_data = jwt::decode_jwt::<AuthClaims>(&cookie).map_err(|e| {
        warn!("decode jwt failed: {e:?}");
        return (StatusCode::UNAUTHORIZED, "Access denied.".to_string());
    });

    match claim_data {
        Err(_) => {
            info!("Attempting to authenticate with API tokens...");
            Ok(try_api_token_authentication(&state, &cookie).await?)
        }
        Ok(claim) => Ok(claim),
    }
}

/// Implement OptionalFromRequestParts for AuthClaims because not every API
/// needs to be strictly authenticated, it just helps if it is. For example, /api/ants/suggest
/// does not require it, but should use it if it's included.
impl<S> OptionalFromRequestParts<S> for AuthClaims
where
    InnerApiState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let cookies = Cookies::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                error!("Failed to parse cookies: {:?}", e);
                return AntOnTheWebError::InternalServerError(None).into();
            })?;

        let cookie = match cookies.get(AUTH_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => {
                info!(
                    "cookie had no '{}' key for optional auth, skipping...",
                    AUTH_COOKIE_NAME
                );
                return Ok(None);
            }
        };

        let state = InnerApiState::from_ref(state);
        let decoded = decode_jwt_cookie(&state, cookie.value()).await?;

        info!("Authentication successful.");
        Ok(Some(decoded))
    }
}

/// Implement FromRequestParts for AuthClaims for APIs that absolutely need to be authenticated,
/// for example, /api/users/user, /api/users/logout, or other profile information.
impl<S> FromRequestParts<S> for AuthClaims
where
    InnerApiState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                error!("Failed to parse cookies: {:?}", e);
                return AntOnTheWebError::InternalServerError(None).into();
            })?;

        let cookie = match cookies.get(AUTH_COOKIE_NAME) {
            Some(cookie) => cookie,
            None => {
                warn!("No '{}' cookie found.", AUTH_COOKIE_NAME);
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid authorization token.".to_string(),
                ));
            }
        };

        let state = InnerApiState::from_ref(state);

        let auth = decode_jwt_cookie(&state, cookie.value()).await?;
        info!("Authentication successful.");
        Ok(auth)
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
            AuthError::AccessDenied(e) => {
                write!(
                    f,
                    "AuthError::AccessDenied::{}",
                    e.clone().unwrap_or("<none>".to_string())
                )
            }
            AuthError::InternalServerError(e) => write!(f, "AuthError::InternalServerError::{}", e),
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
                error!("AuthError::InternalServerError::{}", e);
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
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self::InternalServerError(value.into())
    }
}

/// This function allows any user in that has successfully signed up, but may not have performed
/// their two-factor verification. This should be used for routes required to start/finish the
/// two-factor verification flows.
///
/// Unless this is a route that deals with auth, you should use [`authenticate`].
pub async fn authenticate_weak(
    auth: &AuthClaims,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    info!("Attempting weak authentication.");
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

fn upgrade_weak_auth_to_strong(auth: &AuthClaims, user: User) -> Result<User, AuthError> {
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
    info!("Attempting authentication.");
    return upgrade_weak_auth_to_strong(&auth, authenticate_weak(&auth, &dao).await?);
}

/// Requires that the claim that the user carries belongs to a user with the admin role.
/// Stronger than authenticate(), which just requires that the caller be a valid user.
pub async fn authenticate_admin(
    auth: &AuthClaims,
    dao: &Arc<AntDataFarmClient>,
) -> Result<User, AuthError> {
    info!("Attempting admin authentication.");
    let user = authenticate(&auth, &dao).await?;

    match user.role_name.as_str() {
        "admin" => Ok(user),
        _ => Err(AuthError::AccessDenied(Some(format!(
            "User is not 'admin', instead '{}'",
            user.role_name
        )))),
    }
}

/// If the auth claims are present, return the user that is authenticated. If not, returns None.
/// This is different from [`optional_authenticate`] since that will return the 'nobody' user.
///
/// This function treats WEAK authentication as NO authentication. Returns AccessDenied errors if
/// the user is specified but the claims are tampered or somehow wrong.
///
/// This is used in public APIs, for private user-specific APIs default to using [`authenticate`].
pub async fn optional_strict_authenticate(
    auth: Option<&AuthClaims>,
    dao: &Arc<AntDataFarmClient>,
) -> Result<Option<User>, AuthError> {
    return match auth.map(async |claims| {
        authenticate_weak(&claims, &dao)
            .await
            .ok()
            .and_then(|weak_user| upgrade_weak_auth_to_strong(&claims, weak_user).ok())
    }) {
        None => Ok(None),
        Some(u) => Ok(u.await),
    };
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
    match optional_strict_authenticate(auth, dao).await? {
        None => {
            let users = dao.users.read().await;
            Ok(users
                .get_one_by_user_name("nobody")
                .await?
                .expect("nobody user exists"))
        }
        Some(user) => Ok(user),
    }
}

/// This function should be used to _weakly_ authenticate the user. They might be signed in
/// but they have yet to complete the 2fa flow. They should only be able to access routes
/// for submitting 2fa codes, resending them, or logging out.
pub fn make_weak_auth_cookie(user_id: UserId) -> Result<(String, Cookie<'static>), anyhow::Error> {
    debug!("Making JWT weak cookie for the browser...");
    let claims = AuthClaims::new(user_id);
    let jwt = jwt::encode_jwt(&claims)?;

    let cookie = cookie_defaults(AUTH_COOKIE_NAME, jwt.clone())
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
    let cookie = cookie_defaults(AUTH_COOKIE_NAME, jwt.clone()).build();

    Ok((jwt, cookie))
}

pub fn cookie_defaults(name: &'static str, content: String) -> CookieBuilder<'static> {
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie
    Cookie::build((name, content.clone()))
        .secure(true)
        .http_only(true)
        .permanent()
        .path("/")
        .same_site(match get_mode() {
            Mode::Dev => SameSite::None,
            Mode::Prod => SameSite::Strict,
        })
}
