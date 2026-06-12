use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::{error, warn};

#[derive(Debug)]
pub enum AntArchiveError {
    InternalServerError(Option<anyhow::Error>),
    Unauthorized,
    Forbidden,
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for AntArchiveError {
    fn into_response(self) -> Response {
        match self {
            AntArchiveError::InternalServerError(e) => {
                error!("AntArchiveError::InternalServerError: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
            }
            AntArchiveError::Unauthorized => {
                warn!("AntArchiveError::Unauthorized");
                (StatusCode::UNAUTHORIZED, "Unauthorized.").into_response()
            }
            AntArchiveError::Forbidden => {
                warn!("AntArchiveError::Forbidden");
                (StatusCode::FORBIDDEN, "Forbidden.").into_response()
            }
            AntArchiveError::NotFound(key) => {
                (StatusCode::NOT_FOUND, format!("{key} not found")).into_response()
            }
            AntArchiveError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
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
