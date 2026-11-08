use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AntArchiveError, AntArchiveState, auth};

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub id: String,
    pub key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListObjectsResponse {
    pub objects: Vec<ObjectMetadata>,
}

pub async fn list_objects(
    State(state): State<AntArchiveState>,
    maybe_auth: Option<auth::BearerClaims>,
    Path((bucket_id, key_prefix)): Path<(String, String)>,
) -> Result<Json<ListObjectsResponse>, AntArchiveError> {
    auth::access_bucket_object(&state, maybe_auth, &bucket_id, &key_prefix).await?;

    let objects = state
        .db
        .list_objects_with_prefix(&bucket_id, &key_prefix)
        .await?
        .into_iter()
        .map(|(id, key, created_at)| ObjectMetadata {
            id,
            key,
            created_at,
        })
        .collect();

    Ok(Json(ListObjectsResponse { objects }))
}
