use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "__type")]
pub enum AntPrintingPressError {
    InternalServerError(#[serde(skip)] Option<anyhow::Error>),
    ValidationMessage(String),
}

impl IntoResponse for AntPrintingPressError {
    fn into_response(self) -> Response {
        match self {
            AntPrintingPressError::InternalServerError(e) => {
                error!("AntPrintingPressError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AntPrintingPressError::InternalServerError(None)),
                )
                    .into_response()
            }

            AntPrintingPressError::ValidationMessage(msg) => (
                StatusCode::BAD_REQUEST,
                Json(AntPrintingPressError::ValidationMessage(msg)),
            )
                .into_response(),
        }
    }
}

impl<E> From<E> for AntPrintingPressError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(Some(err.into()))
    }
}
