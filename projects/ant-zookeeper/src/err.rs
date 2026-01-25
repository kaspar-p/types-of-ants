use axum::response::{IntoResponse, Json, Response};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AntZookeeperError {
    InternalServerError(#[serde(skip)] Option<anyhow::Error>),
    ValidationError(String),
    ResourceNotFound(String),
}

impl AntZookeeperError {
    pub fn validation_msg(msg: &str) -> Self {
        Self::ValidationError(msg.to_string())
    }

    pub fn json(self) -> (StatusCode, Json<Self>) {
        match self {
            Self::InternalServerError(_) => {
                error!("Error: {:?}", self);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(self));
            }
            Self::ResourceNotFound(_) | Self::ValidationError(_) => {
                debug!("Error: {:?}", self);
                (StatusCode::BAD_REQUEST, Json(self))
            }
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
        let val: (StatusCode, Json<AntZookeeperError>) = self.into();
        val.into_response()
    }
}

impl Into<(StatusCode, Json<AntZookeeperError>)> for AntZookeeperError {
    fn into(self) -> (StatusCode, Json<AntZookeeperError>) {
        self.json()
    }
}
