use axum_typed_multipart::{FieldData, TryFromMultipart};
use serde::{Deserialize, Serialize};

#[derive(TryFromMultipart)]
pub struct LaunchProjectRequest {
    pub project: String, // Project, but can't put enums in Multipart
    pub artifact: FieldData<axum::body::Bytes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LaunchStatus {
    LaunchSuccessful,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchProjectResponse {
    pub status: LaunchStatus,
}
