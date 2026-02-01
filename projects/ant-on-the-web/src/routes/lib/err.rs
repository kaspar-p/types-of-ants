use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub msg: String,
}

impl ValidationMessage {
    pub fn new<S>(field: &'static str, msg: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            field: Some(field.to_string()),
            msg: msg.into(),
        }
    }

    pub fn invalid(field: &'static str) -> Self {
        Self {
            field: Some(field.to_string()),
            msg: "Field invalid.".to_string(),
        }
    }

    pub fn msg<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            field: None,
            msg: msg.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
// Need to re-tag with __type because newtype structs like Outer(Inner) are serialized as Inner.
#[serde(rename_all = "camelCase", tag = "__type")]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "__type")]
pub enum AntOnTheWebError {
    AccessDenied(#[serde(skip)] Option<String>),
    InternalServerError(#[serde(skip)] Option<anyhow::Error>),
    ValidationError(ValidationError),
    ConflictError { msg: &'static str },
    NoSuchPage { page: i32 },
    NoSuchResource,
}

impl IntoResponse for AntOnTheWebError {
    fn into_response(self) -> Response {
        match self {
            AntOnTheWebError::InternalServerError(e) => {
                error!("AntOnTheWebError::InternalServerError: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AntOnTheWebError::InternalServerError(None)),
                )
                    .into_response()
            }

            AntOnTheWebError::AccessDenied(identity) => {
                warn!("AntOnTheWebError::AccessDenied: {:?}", identity);
                (
                    StatusCode::UNAUTHORIZED,
                    Json(AntOnTheWebError::AccessDenied(None)),
                )
                    .into_response()
            }

            AntOnTheWebError::ValidationError(msg) => {
                warn!("AntOnTheWebError::ValidationError: {:?}", msg);
                (StatusCode::BAD_REQUEST, Json(msg)).into_response()
            }

            AntOnTheWebError::ConflictError { msg } => {
                warn!("AntOnTheWebError::ConflictError: {:?}", msg);
                (StatusCode::CONFLICT, Json(self)).into_response()
            }

            AntOnTheWebError::NoSuchPage { page } => {
                warn!("AntOnTheWebError::NoSuchPage: {:?}", page);
                (StatusCode::NOT_FOUND, Json(self)).into_response()
            }

            AntOnTheWebError::NoSuchResource => {
                warn!("AntOnTheWebError::NoSuchResource");
                (StatusCode::NOT_FOUND, Json(self)).into_response()
            }
        }
    }
}

// impl Into<(StatusCode, String)> for AntOnTheWebError {
//     fn into(self) -> (StatusCode, String) {
//         match self {
//             AntOnTheWebError::InternalServerError(e) => {
//                 error!("AntOnTheWebError::InternalServerError: {:?}", e);
//                 (
//                     StatusCode::INTERNAL_SERVER_ERROR,
//                     "Something went wrong, please retry.".to_string(),
//                 )
//             }

//             AntOnTheWebError::AccessDenied(identity) => {
//                 warn!("AntOnTheWebError::AccessDenied: {:?}", identity);
//                 (StatusCode::UNAUTHORIZED, "Access denied.".to_string())
//             }

//             AntOnTheWebError::Validation(msg) => {
//                 warn!("AntOnTheWebError::ValidationError: {:?}", msg);
//                 panic!("Not supported!")
//             }

//             AntOnTheWebError::ConflictError(taken) => {
//                 warn!("UsersError::ConflictError: {:?}", taken);
//                 (StatusCode::CONFLICT, taken.to_string())
//             }
//         }
//     }
// }

impl<E> From<E> for AntOnTheWebError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(Some(err.into()))
    }
}
