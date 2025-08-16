use ant_data_farm::web_actions::{WebAction, WebTargetType};
use axum::{
    extract::{FromRequestParts, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    err::AntOnTheWebError,
    routes::lib::auth::{cookie_defaults, optional_authenticate, AuthClaims},
    state::{ApiState, InnerApiState},
};

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
/// Saves the web request uri into the database.
pub async fn telemetry_cookie_middleware(
    State(InnerApiState { dao, .. }): ApiState,
    telemetry: TelemetryCookie, // This has to be here to run the extractor.
    auth: Option<AuthClaims>,
    req: Request,
    next: Next,
) -> Response {
    let user = optional_authenticate(auth.as_ref(), &dao)
        .await
        .map_err(|e| {
            error!("error finding user during telemetry: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error, please retry.",
            )
                .into_response();
        });

    let uri = req.uri().to_string().clone();

    match user {
        Ok(u) => {
            tokio::spawn(async move {
                info!("Writing telemetry action for page visit to: {uri}");
                dao.web_actions
                    .write()
                    .await
                    .new_action(
                        telemetry.token,
                        &u.clone().user_id,
                        &WebAction::Visit,
                        &WebTargetType::Page,
                        &uri,
                    )
                    .await
                    .unwrap_or_else(|e| {
                        error!("Writing telemetry failed: {e}");
                    });
            });
        }
        Err(_) => {
            //
        }
    }

    next.run(req).await
}
