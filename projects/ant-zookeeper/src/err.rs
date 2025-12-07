use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::error;

pub enum AntZookeeperError {
    InternalServerError(Option<anyhow::Error>),
}

impl<E> From<E> for AntZookeeperError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(Some(err.into()))
    }
}

impl IntoResponse for AntZookeeperError {
    fn into_response(self) -> Response {
        match self {
            AntZookeeperError::InternalServerError(e) => {
                error!("AntZookeeperError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.",
                )
                    .into_response()
            }
        }
    }
}

impl Into<(StatusCode, String)> for AntZookeeperError {
    fn into(self) -> (StatusCode, String) {
        match self {
            AntZookeeperError::InternalServerError(e) => {
                error!("AntZookeeperError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.".to_string(),
                )
            }
        }
    }
}
