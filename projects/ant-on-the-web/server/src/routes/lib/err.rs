use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, warn};

use super::auth::AuthError;

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub field: String,
    pub msg: String,
}

impl ValidationMessage {
    pub fn new(field: &'static str, msg: &'static str) -> Self {
        Self {
            field: field.to_string(),
            msg: msg.to_string(),
        }
    }

    pub fn invalid(field: &'static str) -> Self {
        Self {
            field: field.to_string(),
            msg: "Field invalid.".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub errors: Vec<ValidationMessage>,
}

pub enum AntOnTheWebError {
    AccessDenied(Option<String>),
    InternalServerError(anyhow::Error),
    ValidationError(ValidationError),
    ConflictError(&'static str),
}

impl IntoResponse for AntOnTheWebError {
    fn into_response(self) -> Response {
        match self {
            AntOnTheWebError::InternalServerError(e) => {
                error!("AntOnTheWebError::InternalServerError {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.",
                )
                    .into_response()
            }

            AntOnTheWebError::AccessDenied(identity) => {
                warn!("AntOnTheWebError::AccessDenied, identity: {:?}", identity);
                (StatusCode::UNAUTHORIZED, "Access denied.").into_response()
            }

            AntOnTheWebError::ValidationError(msg) => {
                warn!("AntOnTheWebError::ValidationError {:?}", msg);
                (StatusCode::BAD_REQUEST, Json(msg)).into_response()
            }

            AntOnTheWebError::ConflictError(taken) => {
                warn!("UsersError::ConflictError {:?}", taken);
                (StatusCode::CONFLICT, taken).into_response()
            }
        }
    }
}

impl From<AuthError> for AntOnTheWebError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::AccessDenied(e) => AntOnTheWebError::AccessDenied(e),
            AuthError::InternalServerError(e) => AntOnTheWebError::InternalServerError(e),
        }
    }
}

impl<E> From<E> for AntOnTheWebError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(err.into())
    }
}
