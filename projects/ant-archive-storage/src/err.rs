use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::{error, warn};

#[derive(Debug)]
pub enum AntArchiveStorageError {
    InternalServerError(&'static str, Option<anyhow::Error>),
    AccessDenied,
    NotFound(String),
    RangeNotSatisfiable,
    BadRequest(String),
}

impl IntoResponse for AntArchiveStorageError {
    fn into_response(self) -> Response {
        match self {
            AntArchiveStorageError::InternalServerError(id, e) => {
                error!("ANT-ERR-004: {id}: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
            }
            AntArchiveStorageError::AccessDenied => {
                warn!("AntArchiveStorageError::AccessDenied");
                (StatusCode::UNAUTHORIZED, "Access denied.").into_response()
            }
            AntArchiveStorageError::NotFound(key) => {
                (StatusCode::NOT_FOUND, format!("{key} not found")).into_response()
            }
            AntArchiveStorageError::RangeNotSatisfiable => {
                (StatusCode::RANGE_NOT_SATISFIABLE, "Range not satisfiable.").into_response()
            }
            AntArchiveStorageError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
        }
    }
}

impl<E> From<E> for AntArchiveStorageError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError("?", Some(err.into()))
    }
}
