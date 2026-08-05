use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use http::StatusCode;

use crate::{auth::BearerClaims, err::AntArchiveError, state::AntArchiveState};

pub(super) async fn delete_object(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    auth: BearerClaims,
) -> Result<impl IntoResponse, AntArchiveError> {
    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::BucketNotFound(bucket_id.clone()))?;

    if bucket.client_id != auth.client_id {
        return Err(AntArchiveError::BucketNotFound(bucket_id.clone()));
    }

    let _ = state
        .db
        .soft_delete_key(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::ObjectNotFound(key.clone()))?;

    Ok(StatusCode::OK)
}
