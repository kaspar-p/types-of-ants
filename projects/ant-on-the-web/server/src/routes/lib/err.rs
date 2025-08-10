use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
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

impl ValidationError {
    pub fn one(error: ValidationMessage) -> Self {
        ValidationError {
            errors: vec![error],
        }
    }
    pub fn many(errors: Vec<ValidationMessage>) -> Self {
        ValidationError { errors }
    }
}

#[derive(Debug)]
pub enum AntOnTheWebError {
    AccessDenied(Option<String>),
    InternalServerError(Option<anyhow::Error>),
    Validation(ValidationError),
    ConflictError(&'static str),
}

impl IntoResponse for AntOnTheWebError {
    fn into_response(self) -> Response {
        match self {
            AntOnTheWebError::InternalServerError(e) => {
                error!("AntOnTheWebError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.",
                )
                    .into_response()
            }

            AntOnTheWebError::AccessDenied(identity) => {
                warn!("AntOnTheWebError::AccessDenied: {:?}", identity);
                (StatusCode::UNAUTHORIZED, "Access denied.").into_response()
            }

            AntOnTheWebError::Validation(msg) => {
                warn!("AntOnTheWebError::ValidationError: {:?}", msg);
                (StatusCode::BAD_REQUEST, Json(msg)).into_response()
            }

            AntOnTheWebError::ConflictError(taken) => {
                warn!("UsersError::ConflictError: {:?}", taken);
                (StatusCode::CONFLICT, taken).into_response()
            }
        }
    }
}

impl Into<(StatusCode, String)> for AntOnTheWebError {
    fn into(self) -> (StatusCode, String) {
        match self {
            AntOnTheWebError::InternalServerError(e) => {
                error!("AntOnTheWebError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.".to_string(),
                )
            }

            AntOnTheWebError::AccessDenied(identity) => {
                warn!("AntOnTheWebError::AccessDenied: {:?}", identity);
                (StatusCode::UNAUTHORIZED, "Access denied.".to_string())
            }

            AntOnTheWebError::Validation(msg) => {
                warn!("AntOnTheWebError::ValidationError: {:?}", msg);
                panic!("Not supported!")
            }

            AntOnTheWebError::ConflictError(taken) => {
                warn!("UsersError::ConflictError: {:?}", taken);
                (StatusCode::CONFLICT, taken.to_string())
            }
        }
    }
}

impl From<AuthError> for AntOnTheWebError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::AccessDenied(e) => AntOnTheWebError::AccessDenied(e),
            AuthError::InternalServerError(e) => AntOnTheWebError::InternalServerError(Some(e)),
        }
    }
}

impl<E> From<E> for AntOnTheWebError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(Some(err.into()))
    }
}
