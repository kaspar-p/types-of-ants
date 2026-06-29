use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::error;

#[derive(Debug)]
pub enum AntArchiveError {
    InternalServerError(&'static str, Option<anyhow::Error>),
    Unauthorized(Option<anyhow::Error>),
    BucketNotFound(String),
    ObjectNotFound(String),
    BadRequest(String),
    InsufficientStorage,
}

impl IntoResponse for AntArchiveError {
    fn into_response(self) -> Response {
        match self {
            AntArchiveError::InternalServerError(id, e) => {
                error!("ANT-ERR-001: {id}: {:?}", e);
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
            AntArchiveError::InsufficientStorage => {
                (StatusCode::INSUFFICIENT_STORAGE, "Insufficient storage capacity.").into_response()
            }
        }
    }
}

impl From<anyhow::Error> for AntArchiveError {
    fn from(err: anyhow::Error) -> Self {
        Self::InternalServerError("?", Some(err))
    }
}

impl From<ant_archive_db::AntArchiveDbError> for AntArchiveError {
    fn from(e: ant_archive_db::AntArchiveDbError) -> Self {
        let code = match &e {
            ant_archive_db::AntArchiveDbError::Connection(_) => "ANT-ERR-129",
            ant_archive_db::AntArchiveDbError::Query(_) => "ANT-ERR-130",
        };
        Self::InternalServerError(code, Some(e.into()))
    }
}
