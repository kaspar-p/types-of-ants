use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use anyhow::Context;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Router,
};
use http::{StatusCode, header};
use http_body_util::BodyExt;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer};

use crate::{auth::BearerClaims, err::AntArchiveError, state::AntArchiveState};

fn encrypt_blob(
    kek: &[u8; 32],
    plaintext: &[u8],
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), AntArchiveError> {
    let mut dek = [0u8; 32];
    OsRng.fill_bytes(&mut dek);

    let blob_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let dek_key = Key::<Aes256Gcm>::from_slice(&dek);
    let blob_cipher = Aes256Gcm::new(dek_key);
    let ciphertext = blob_cipher.encrypt(&blob_nonce, plaintext).map_err(|_| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("blob encryption failed")))
    })?;

    let mut stored_bytes = blob_nonce.to_vec();
    stored_bytes.extend_from_slice(&ciphertext);

    let dek_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let encrypted_dek = kek_cipher.encrypt(&dek_nonce, dek.as_ref()).map_err(|_| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("DEK encryption failed")))
    })?;

    Ok((encrypted_dek, dek_nonce.to_vec(), stored_bytes))
}

fn decrypt_blob(
    kek: &[u8; 32],
    encrypted_dek: &[u8],
    dek_nonce_bytes: &[u8],
    stored_bytes: &[u8],
) -> Result<Vec<u8>, AntArchiveError> {
    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let dek_nonce = Nonce::from_slice(dek_nonce_bytes);
    let dek = kek_cipher
        .decrypt(dek_nonce, encrypted_dek)
        .map_err(|_| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!("DEK decryption failed")))
        })?;

    if stored_bytes.len() < 12 {
        return Err(AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
            "stored bytes too short to contain nonce"
        ))));
    }
    let (blob_nonce_bytes, ciphertext) = stored_bytes.split_at(12);

    let dek_arr: [u8; 32] = dek.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("DEK wrong length")))
    })?;
    let dek_key = Key::<Aes256Gcm>::from_slice(&dek_arr);
    let blob_cipher = Aes256Gcm::new(dek_key);
    let blob_nonce = Nonce::from_slice(blob_nonce_bytes);

    blob_cipher.decrypt(blob_nonce, ciphertext).map_err(|_| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("blob decryption failed")))
    })
}

fn compute_checksum(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    base16ct::lower::encode_string(&hash)
}

async fn put_blob(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    auth: BearerClaims,
    body: Body,
) -> Result<impl IntoResponse, AntArchiveError> {
    if key.starts_with('/') || key.is_empty() {
        return Err(AntArchiveError::BadRequest(
            "key must not be empty or start with '/'".to_string(),
        ));
    }

    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::NotFound(format!("bucket {bucket_id}")))?;

    if bucket.client_id != auth.client_id {
        return Err(AntArchiveError::Forbidden);
    }

    let plaintext = body
        .collect()
        .await
        .context("failed to read request body")?
        .to_bytes()
        .to_vec();
    let plaintext_len = plaintext.len() as i64;

    let (encrypted_dek, dek_nonce, stored_bytes) = encrypt_blob(state.kek(), &plaintext)?;
    let checksum = compute_checksum(&stored_bytes);

    let storage_node = state.storage_nodes.first().ok_or_else(|| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
            "no storage nodes configured"
        )))
    })?;

    let blob_id = state
        .db
        .upsert_blob(
            &bucket_id,
            &state.kek_id,
            &key,
            plaintext_len,
            &encrypted_dek,
            &dek_nonce,
        )
        .await?;

    storage_node.put(&blob_id, stored_bytes).await?;

    state
        .db
        .upsert_placement(&blob_id, &storage_node.node_id, &blob_id, &checksum)
        .await?;

    Ok(StatusCode::CREATED)
}

async fn get_blob(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    maybe_auth: Option<BearerClaims>,
) -> Result<Response, AntArchiveError> {
    if key.starts_with('/') || key.is_empty() {
        return Err(AntArchiveError::BadRequest(
            "key must not be empty or start with '/'".to_string(),
        ));
    }

    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::NotFound(format!("bucket {bucket_id}")))?;

    match bucket.read_policy.as_str() {
        "public" => {}
        "internal" => {
            if maybe_auth.is_none() {
                return Err(AntArchiveError::Unauthorized);
            }
        }
        "private" => {
            // Return NotFound for both missing and wrong auth to prevent bucket enumeration.
            let not_found = || AntArchiveError::NotFound(format!("{bucket_id}/{key}"));
            let auth = maybe_auth.ok_or_else(&not_found)?;
            if bucket.client_id != auth.client_id {
                return Err(not_found());
            }
        }
        _ => {
            return Err(AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "unknown read policy"
            ))))
        }
    }

    let blob = state
        .db
        .get_blob(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::NotFound(format!("{bucket_id}/{key}")))?;

    let placements = state.db.get_placements(&blob.blob_id).await?;
    let placement = placements.first().ok_or_else(|| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("no placements for blob")))
    })?;

    let storage_node = state
        .storage_nodes
        .iter()
        .find(|n| n.node_id == placement.storage_node_id)
        .ok_or_else(|| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "storage node not in state"
            )))
        })?;

    let stored_bytes = storage_node
        .get(&placement.storage_key)
        .await?
        .ok_or_else(|| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "blob missing from storage"
            )))
        })?;

    let plaintext =
        decrypt_blob(state.kek(), &blob.encrypted_dek, &blob.dek_nonce, &stored_bytes)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, plaintext.len())
        .body(Body::from(plaintext))
        .context("failed to build response")?)
}

async fn delete_blob(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    auth: BearerClaims,
) -> Result<impl IntoResponse, AntArchiveError> {
    if key.starts_with('/') || key.is_empty() {
        return Err(AntArchiveError::BadRequest(
            "key must not be empty or start with '/'".to_string(),
        ));
    }

    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::NotFound(format!("bucket {bucket_id}")))?;

    if bucket.client_id != auth.client_id {
        return Err(AntArchiveError::Forbidden);
    }

    let blob = state
        .db
        .get_blob(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::NotFound(format!("{bucket_id}/{key}")))?;

    let placements = state.db.get_placements(&blob.blob_id).await?;
    for placement in &placements {
        if let Some(storage_node) = state
            .storage_nodes
            .iter()
            .find(|n| n.node_id == placement.storage_node_id)
        {
            storage_node.delete(&placement.storage_key).await?;
        }
    }

    state.db.soft_delete_blob(&bucket_id, &key).await?;

    Ok(StatusCode::OK)
}

pub fn make_routes(state: AntArchiveState) -> Result<Router, anyhow::Error> {
    use ant_library::routes::Routes;

    let app = Routes::new()
        .put("/{bucket_id}/{*key}", put(put_blob))
        .get("/{bucket_id}/{*key}", get(get_blob))
        .delete("/{bucket_id}/{*key}", delete(delete_blob))
        .build()
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(ant_library::middleware::catch_panic))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                )))
                .layer(DefaultBodyLimit::disable())
                .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024)),
        );

    Ok(app)
}
