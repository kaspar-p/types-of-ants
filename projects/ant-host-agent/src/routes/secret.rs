use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::io::ErrorKind;
use tracing::error;

use crate::state::AntHostAgentState;

#[serde_as]
#[derive(Deserialize, Serialize)]
pub struct PutSecretRequest {
    pub name: String,
    #[serde_as(as = "Base64")]
    pub value: Vec<u8>,
}

async fn put_secret(
    State(state): State<AntHostAgentState>,
    Json(req): Json<PutSecretRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let value = req.value.to_vec();

    let file = state
        .secrets_root_dir
        .join(ant_library::secret::secret_name(req.name.as_str()));

    std::fs::write(&file, value).map_err(|e| {
        error!(
            "Failed to write to secrets file [{}]: {}",
            &file.display(),
            e
        );

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error, please retry.".to_string(),
        )
    })?;

    Ok((StatusCode::OK, "Secret received.".to_string()))
}

#[derive(Deserialize, Serialize)]
pub struct DeleteSecretRequest {
    pub secret_name: String,
}
async fn delete_secret(
    State(state): State<AntHostAgentState>,
    Json(req): Json<DeleteSecretRequest>,
) -> impl IntoResponse {
    let file = state
        .secrets_root_dir
        .join(ant_library::secret::secret_name(req.secret_name.as_str()));
    let res = match std::fs::remove_file(file) {
        Ok(_) => StatusCode::OK,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => StatusCode::OK,
            _ => {
                error!("Failed to delete secret: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        },
    };

    res
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PeekSecretRequest {
    pub secret_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PeekSecretResponse {
    pub secret_exists: bool,
}

async fn peek_secret(
    State(state): State<AntHostAgentState>,
    Json(req): Json<PeekSecretRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let file = state
        .secrets_root_dir
        .join(ant_library::secret::secret_name(req.secret_name.as_str()));
    let exists = std::fs::exists(file).map_err(|e| {
        error!("Unknown error: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error, please retry.",
        )
    })?;

    Ok((
        StatusCode::OK,
        Json(PeekSecretResponse {
            secret_exists: exists,
        }),
    ))
}

pub fn make_routes() -> Router<AntHostAgentState> {
    Router::new()
        .route_with_tsr(
            "/secret",
            post(put_secret).delete(delete_secret).get(peek_secret),
        )
        .fallback(|| async {
            ant_library::api_fallback(&[
                "GET /secret/secret",
                "POST /secret/secret",
                "DELETE /secret/secret",
            ])
        })
}
