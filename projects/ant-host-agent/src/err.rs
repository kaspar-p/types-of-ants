use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use tracing::{debug, error};

pub enum AntHostAgentError {
    InternalServerError(Option<anyhow::Error>),
    ValidationError {
        msg: String,
        e: Option<anyhow::Error>,
    },
}

impl AntHostAgentError {
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

impl<E> From<E> for AntHostAgentError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(Some(err.into()))
    }
}

impl IntoResponse for AntHostAgentError {
    fn into_response(self) -> Response {
        let val: (StatusCode, String) = self.into();
        val.into_response()
    }
}

impl Into<(StatusCode, String)> for AntHostAgentError {
    fn into(self) -> (StatusCode, String) {
        match self {
            AntHostAgentError::InternalServerError(e) => {
                error!("AntHostAgentError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry.".to_string(),
                )
            }
            AntHostAgentError::ValidationError { msg, e } => {
                debug!(
                    "AntHostAgentError::ValidationError: {:?} caused by {:?}",
                    msg, e
                );

                (StatusCode::BAD_REQUEST, msg)
            }
        }
    }
}
