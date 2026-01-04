use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::{debug, error};

pub enum AntZookeeperError {
    InternalServerError(Option<anyhow::Error>),
    ValidationError {
        msg: String,
        e: Option<anyhow::Error>,
    },
}

impl AntZookeeperError {
    pub fn validation_msg(msg: &str) -> Self {
        Self::ValidationError {
            msg: msg.to_string(),
            e: None,
        }
    }

    pub fn validation(msg: &str, e: Option<anyhow::Error>) -> Self {
        Self::ValidationError {
            msg: msg.to_string(),
            e,
        }
    }
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
        let val: (StatusCode, String) = self.into();
        val.into_response()
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

            AntZookeeperError::ValidationError { msg, e } => {
                debug!(
                    "AntZookeeperError::ValidationError: {:?} caused by {:?}",
                    msg, e
                );

                (StatusCode::BAD_REQUEST, msg)
            }
        }
    }
}
