use ant_data_farm::web_actions::{WebAction, WebTargetType};
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    err::AntOnTheWebError,
    routes::lib::{
        auth::{optional_authenticate, AuthClaims},
        telemetry::TelemetryCookie,
    },
    state::{ApiRouter, ApiState, InnerApiState},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct WebActionRequest {
    pub action: WebAction,

    #[serde(rename = "targetType")]
    pub target_type: WebTargetType,

    pub target: String,
}

async fn new_web_action(
    tracking: TelemetryCookie,
    auth: Option<AuthClaims>,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<WebActionRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = optional_authenticate(auth.as_ref(), &dao).await?;

    dao.web_actions
        .write()
        .await
        .new_action(
            tracking.token,
            &user.user_id,
            &req.action,
            &req.target_type,
            &req.target,
        )
        .await?;

    Ok((StatusCode::OK, "Action received."))
}

pub fn router() -> ApiRouter {
    Router::new().route_with_tsr("/action", post(new_web_action))
}
