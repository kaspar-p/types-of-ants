use ant_data_farm::web_actions::{WebAction, WebTargetType};
use ant_library::routes::Routes;
use axum::{extract::State, routing::post, Json};
use serde::{Deserialize, Serialize};

use crate::{
    err::AntOnTheWebError,
    routes::lib::{
        auth::{optional_authenticate, AuthClaims},
        response::AntOnTheWebResponse,
        telemetry::TelemetryCookie,
    },
    state::{ApiRoutes, ApiState, InnerApiState},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebActionRequest {
    pub action: WebAction,
    pub target_type: WebTargetType,
    pub target: String,
}

async fn new_web_action(
    tracking: TelemetryCookie,
    auth: Option<AuthClaims>,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<WebActionRequest>,
) -> Result<AntOnTheWebResponse, AntOnTheWebError> {
    let user = optional_authenticate(auth.as_ref(), &dao).await?;

    dao.web_actions
        .new_action(
            tracking.token,
            &user.user_id,
            &req.action,
            &req.target_type,
            &req.target,
        )
        .await?;

    Ok(AntOnTheWebResponse::WebActionResponse)
}

pub fn routes() -> ApiRoutes {
    Routes::new().post("/action", post(new_web_action))
}
