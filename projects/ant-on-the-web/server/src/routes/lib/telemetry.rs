use axum::{
    extract::{FromRequestParts, Request},
    middleware::Next,
    response::Response,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use tracing::error;
use uuid::Uuid;

use crate::{err::AntOnTheWebError, routes::lib::auth::cookie_defaults};

/// This cookie contains encrypted information on who the user is. The encryption is purely symmetric for anti-tampering.
pub const TELEMETRY_COOKIE_NAME: &str = "typesofants_telemetry";

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryCookie {
    pub token: Uuid,
}

impl TelemetryCookie {
    pub fn new(token: Uuid) -> Self {
        TelemetryCookie { token: token }
    }
}

/// Implement OptionalFromRequestParts for TelemetryCookie because not every API
/// needs to be strictly authenticated, it just helps if it is. For example, /api/ants/suggest
/// does not require it, but should use it if it's included.
impl<S> FromRequestParts<S> for TelemetryCookie
where
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

        let telemetry = match cookies.get(TELEMETRY_COOKIE_NAME) {
            None => TelemetryCookie::new(Uuid::new_v4()),
            Some(c) => {
                serde_json::from_str(c.value()).unwrap_or(TelemetryCookie::new(Uuid::new_v4()))
            }
        };

        let c = serde_json::to_string(&telemetry).expect("telemetry cookie invalid json");
        cookies.add(cookie_defaults(TELEMETRY_COOKIE_NAME, c).build());

        return Ok(telemetry);
    }
}

/// Propagate the cookie associated with telemetry. If there isn't one, generate one and set it.
pub async fn telemetry_cookie_middleware(
    _telemetry: TelemetryCookie, // This has to be here to run the extractor.
    req: Request,
    next: Next,
) -> Response {
    next.run(req).await
}
