use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use serde::{Deserialize, Serialize};
use tracing::error;

fn default_error_id() -> &'static str {
    ""
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "__type")]
pub enum AntPrintingPressError {
    InternalServerError {
        #[serde(skip, default = "default_error_id")]
        id: &'static str,
        #[serde(skip)]
        err: Option<anyhow::Error>,
    },
    ValidationMessage(String),
}

impl IntoResponse for AntPrintingPressError {
    fn into_response(self) -> Response {
        match self {
            AntPrintingPressError::InternalServerError { id, err: e } => {
                error!("ANT-ERR-066: {id}: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AntPrintingPressError::InternalServerError { id: "ANT-ERR-125", err: None }),
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
        Self::InternalServerError { id: "?", err: Some(err.into()) }
    }
}
