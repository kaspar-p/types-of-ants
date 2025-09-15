use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use chrono::{DateTime, Utc};
use http::StatusCode;
use rand::distr::SampleString;
use serde::{Deserialize, Serialize};

use crate::{
    err::AntOnTheWebError,
    routes::lib::auth::{admin_authenticate, AuthClaims},
    state::{ApiRouter, ApiState, InnerApiState},
};

#[derive(Serialize, Deserialize)]
pub struct GrantTokenRequest {
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct GrantTokenResponse {
    pub token: String,
}

async fn grant_token(
    auth: AuthClaims,
    State(InnerApiState { dao, rng, .. }): ApiState,
    Json(req): Json<GrantTokenRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    admin_authenticate(&auth, &dao).await?;

    let user = dao
        .users
        .read()
        .await
        .get_one_by_user_name(&req.username)
        .await?;
    if user.is_none() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let user = user.unwrap();

    let dist = rand::distr::Alphanumeric;
    let mut rng = rng.lock().await;
    let token = dist.sample_string(&mut rng, 32);

    dao.api_tokens
        .write()
        .await
        .register_api_token(&user.user_id, &token)
        .await?;

    return Ok((StatusCode::OK, Json(GrantTokenResponse { token })).into_response());
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSummary {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct ListTokensResponse {
    pub tokens: Vec<TokenSummary>,
}

async fn list_tokens() -> Result<impl IntoResponse, AntOnTheWebError> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

pub fn router() -> ApiRouter {
    Router::new()
        .route_with_tsr("/tokens", get(list_tokens))
        .route_with_tsr("/token", post(grant_token))
}
