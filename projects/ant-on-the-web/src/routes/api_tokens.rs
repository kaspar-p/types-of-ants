use ant_library::rng::RandAdapter;
use ant_library::routes::Routes;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use chrono::{DateTime, Utc};
use http::StatusCode;
use rand::distr::SampleString;
use serde::{Deserialize, Serialize};

use crate::{
    err::AntOnTheWebError,
    routes::lib::auth::{authenticate_admin, AuthClaims},
    state::{ApiRoutes, ApiState, InnerApiState},
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
    authenticate_admin(&auth, &dao).await?;

    let user = dao.users.get_one_by_user_name(&req.username).await?;
    if user.is_none() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let user = user.unwrap();

    let token = rand::distr::Alphanumeric.sample_string(&mut RandAdapter(&*rng), 32);

    dao.api_tokens
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

pub fn routes() -> ApiRoutes {
    Routes::new()
        .get("/tokens", get(list_tokens))
        .post("/token", post(grant_token))
}
