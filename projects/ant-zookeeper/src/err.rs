use axum::response::{IntoResponse, Json, Response};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

fn default_error_id() -> &'static str {
    ""
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AntZookeeperError {
    InternalServerError {
        #[serde(skip, default = "default_error_id")]
        id: &'static str,
        #[serde(skip)]
        err: Option<anyhow::Error>,
    },
    ValidationError(String),
    ResourceNotFound(String),
}

impl AntZookeeperError {
    pub fn validation_msg(msg: &str) -> Self {
        Self::ValidationError(msg.to_string())
    }

    pub fn json(self) -> (StatusCode, Json<Self>) {
        match self {
            Self::InternalServerError { id, err } => {
                error!("ANT-ERR-070: {id}: {:?}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(Self::InternalServerError { id, err }));
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
        Self::InternalServerError { id: "?", err: Some(err.into()) }
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
