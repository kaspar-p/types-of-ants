use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::error;

#[derive(Debug)]
pub enum AntArchiveError {
    InternalServerError(Option<anyhow::Error>),
    Unauthorized(Option<anyhow::Error>),
    BucketNotFound(String),
    ObjectNotFound(String),
    BadRequest(String),
}

impl IntoResponse for AntArchiveError {
    fn into_response(self) -> Response {
        match self {
            AntArchiveError::InternalServerError(e) => {
                error!("AntArchiveError::InternalServerError: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
            }
            AntArchiveError::Unauthorized(e) => {
                if let Some(e) = e {
                    tracing::debug!("AntArchiveError::Unauthorized: {e:?}");
                }
                (StatusCode::UNAUTHORIZED, "Unauthorized.").into_response()
            }
            AntArchiveError::BucketNotFound(bucket) => {
                (StatusCode::NOT_FOUND, format!("bucket {bucket} not found")).into_response()
            }
            AntArchiveError::ObjectNotFound(key) => {
                (StatusCode::NOT_FOUND, format!("object {key} not found")).into_response()
            }
            AntArchiveError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        }
    }
}

impl<E> From<E> for AntArchiveError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(Some(err.into()))
    }
}
