use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Context;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Router,
};
use base64ct::{Base64, Encoding};
use hkdf::Hkdf;
use http::{header, StatusCode};
use http_body_util::BodyExt;
use rand::RngCore;
use sha2::{Digest, Sha256};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer};
use tracing::{error, info};

use crate::{
    auth::BearerClaims,
    err::AntArchiveError,
    headers::SelectStorageNode,
    placement::{self, resolve_storage_nodes},
    state::AntArchiveState,
};

fn load_kek(kek_id: &str, kek_alias: Option<&str>) -> Result<[u8; 32], AntArchiveError> {
    // ant_archive_kek contains one entry per line: "{kek_alias}:{base64(32 bytes)}"
    let content = ant_library::secret::load_secret("ant_archive_kek")?;
    for line in content.lines() {
        let Some((id, b64)) = line.split_once(':') else {
            continue;
        };

        match kek_alias {
            Some(kek_alias) if id != kek_id && id != kek_alias => {
                continue;
            }
            _ => {}
        }

        let bytes = Base64::decode_vec(b64).map_err(|e| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-091",
                Some(anyhow::anyhow!(
                    "ant_archive_kek entry for '{kek_alias:?}' or '{kek_id}' is not valid base64: {e}"
                )),
            )
        })?;
        let len = bytes.len();
        return bytes.try_into().map_err(|_| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-092",
                Some(anyhow::anyhow!(
                    "ant_archive_kek entry for '{kek_alias:?}' or '{kek_id}' must be exactly 32 bytes, got {len}"
                )),
            )
        });
    }
    Err(AntArchiveError::InternalServerError(
        "ANT-ERR-093",
        Some(anyhow::anyhow!(
            "ant_archive_kek has no entry for kek_id '{kek_alias:?}' or '{kek_id}'"
        )),
    ))
}

fn load_tek_master() -> Result<[u8; 32], AntArchiveError> {
    let bytes = ant_library::secret::load_secret_binary("ant_archive_tek")?;
    let len = bytes.len();
    bytes.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-094",
            Some(anyhow::anyhow!("tek must be exactly 32 bytes, got {len}")),
        )
    })
}

fn generate_tek_derivation_key(rng: &dyn ant_library::rng::Rng) -> [u8; 32] {
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    key
}

fn derive_tek(
    tek_master: &[u8; 32],
    tek_derivation_key: &[u8],
) -> Result<[u8; 32], AntArchiveError> {
    let hkdf = Hkdf::<Sha256>::new(None, tek_master);
    let mut tek = [0u8; 32];
    hkdf.expand(tek_derivation_key, &mut tek).map_err(|e| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-095",
            Some(anyhow::anyhow!("TEK derivation failed: {e}")),
        )
    })?;
    Ok(tek)
}

fn encrypt_object(
    kek: &[u8; 32],
    tek: &[u8; 32],
    plaintext: &[u8],
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), AntArchiveError> {
    let mut dek = [0u8; 32];
    OsRng.fill_bytes(&mut dek);

    let object_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let dek_key = Key::<Aes256Gcm>::from_slice(&dek);
    let object_cipher = Aes256Gcm::new(dek_key);
    let ciphertext = object_cipher
        .encrypt(&object_nonce, plaintext)
        .map_err(|e| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-096",
                Some(anyhow::anyhow!("object encryption failed: {e}")),
            )
        })?;

    let mut inner = object_nonce.to_vec();
    inner.extend_from_slice(&ciphertext);

    let dek_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let encrypted_dek = kek_cipher.encrypt(&dek_nonce, dek.as_ref()).map_err(|e| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-097",
            Some(anyhow::anyhow!("DEK encryption failed: {e}")),
        )
    })?;

    let tek_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let tek_key = Key::<Aes256Gcm>::from_slice(tek);
    let tek_cipher = Aes256Gcm::new(tek_key);
    let outer_ciphertext = tek_cipher
        .encrypt(&tek_nonce, inner.as_ref())
        .map_err(|e| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-098",
                Some(anyhow::anyhow!("TEK encryption failed: {e}")),
            )
        })?;

    let mut stored_bytes = tek_nonce.to_vec();
    stored_bytes.extend_from_slice(&outer_ciphertext);

    Ok((encrypted_dek, dek_nonce.to_vec(), stored_bytes))
}

fn decrypt_object(
    kek: &[u8; 32],
    tek: Option<&[u8; 32]>,
    encrypted_dek: &[u8],
    dek_nonce_bytes: &[u8],
    stored_bytes: &[u8],
) -> Result<Vec<u8>, AntArchiveError> {
    // Pre-TEK objects (tek is None) have stored_bytes = inner blob directly.
    // Post-TEK objects have stored_bytes = tek_nonce || AES-GCM(inner, tek).
    let inner: Vec<u8> = match tek {
        Some(tek) => {
            if stored_bytes.len() < 12 {
                return Err(AntArchiveError::InternalServerError(
                    "ANT-ERR-099",
                    Some(anyhow::anyhow!(
                        "stored bytes too short to contain TEK nonce: {} bytes",
                        stored_bytes.len()
                    )),
                ));
            }
            let (tek_nonce_bytes, outer_ciphertext) = stored_bytes.split_at(12);
            let tek_key = Key::<Aes256Gcm>::from_slice(tek);
            let tek_cipher = Aes256Gcm::new(tek_key);
            let tek_nonce = Nonce::from_slice(tek_nonce_bytes);
            tek_cipher
                .decrypt(tek_nonce, outer_ciphertext)
                .map_err(|e| {
                    AntArchiveError::InternalServerError(
                        "ANT-ERR-100",
                        Some(anyhow::anyhow!("TEK decryption failed: {e}")),
                    )
                })?
        }
        None => stored_bytes.to_vec(),
    };

    if inner.len() < 12 {
        return Err(AntArchiveError::InternalServerError(
            "ANT-ERR-101",
            Some(anyhow::anyhow!(
                "inner blob too short to contain nonce: {} bytes",
                inner.len()
            )),
        ));
    }
    let (object_nonce_bytes, ciphertext) = inner.split_at(12);

    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let dek_nonce = Nonce::from_slice(dek_nonce_bytes);
    let dek = kek_cipher.decrypt(dek_nonce, encrypted_dek).map_err(|e| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-102",
            Some(anyhow::anyhow!("DEK decryption failed: {e}")),
        )
    })?;

    let dek_len = dek.len();
    let dek_arr: [u8; 32] = dek.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-103",
            Some(anyhow::anyhow!(
                "DEK wrong length: expected 32 bytes, got {dek_len}"
            )),
        )
    })?;
    let dek_key = Key::<Aes256Gcm>::from_slice(&dek_arr);
    let object_cipher = Aes256Gcm::new(dek_key);
    let object_nonce = Nonce::from_slice(object_nonce_bytes);

    object_cipher
        .decrypt(object_nonce, ciphertext)
        .map_err(|e| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-104",
                Some(anyhow::anyhow!("object decryption failed: {e}")),
            )
        })
}

fn validate_key(key: &str) -> Result<(), AntArchiveError> {
    if key.is_empty() {
        return Err(AntArchiveError::BadRequest(
            "key must not be empty".to_string(),
        ));
    }
    if key.starts_with('/') {
        return Err(AntArchiveError::BadRequest(
            "key must not start with '/'".to_string(),
        ));
    }
    Ok(())
}

fn compute_checksum(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    base16ct::lower::encode_string(&hash)
}

async fn put_object(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    auth: BearerClaims,
    select_node: Option<SelectStorageNode>,
    body: Body,
) -> Result<impl IntoResponse, AntArchiveError> {
    validate_key(&key)?;

    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::BucketNotFound(bucket_id.clone()))?;

    if bucket.client_id != auth.client_id {
        return Err(AntArchiveError::BucketNotFound(bucket_id.clone()));
    }

    let plaintext = body
        .collect()
        .await
        .context("failed to read request body")?
        .to_bytes()
        .to_vec();
    let plaintext_len = plaintext.len() as i64;

    let (kek_id, kek_alias) = state.db.get_active_kek().await?.ok_or_else(|| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-105",
            Some(anyhow::anyhow!("no active KEK version")),
        )
    })?;

    let tek_master = load_tek_master()?;

    // Reuse the existing tek_derivation_key on overwrites so a storage failure
    // can never leave the DB pointing at a key that doesn't match the stored blob.
    let tek_derivation_key: [u8; 32] = match state
        .db
        .get_object(&bucket_id, &key)
        .await?
        .and_then(|o| o.tek_derivation_key)
    {
        Some(existing) => existing.try_into().map_err(|_| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-106",
                Some(anyhow::anyhow!(
                    "stored tek_derivation_key has unexpected length"
                )),
            )
        })?,
        None => generate_tek_derivation_key(&*state.rng),
    };

    let tek = derive_tek(&tek_master, &tek_derivation_key)?;
    let (encrypted_dek, dek_nonce, stored_bytes) =
        encrypt_object(&load_kek(&kek_id, kek_alias.as_deref())?, &tek, &plaintext)?;
    let checksum = compute_checksum(&stored_bytes);

    let placements = placement::place_new_object(
        &state,
        plaintext_len,
        select_node.as_ref().map(|n| n.0.as_str()),
    )
    .await?;

    let object_id = state
        .db
        .upsert_object(
            &bucket_id,
            &kek_id,
            &key,
            plaintext_len,
            &encrypted_dek,
            &dek_nonce,
            &tek_derivation_key,
        )
        .await?;

    info!(
        "Preparing to send {} bytes to {} nodes",
        stored_bytes.len(),
        placements.len()
    );
    let shared_bytes = bytes::Bytes::from(stored_bytes);
    for (idx, placement) in placements.iter().enumerate() {
        info!(
            "[obj={}] Putting {} bytes to {}",
            &object_id,
            shared_bytes.len(),
            placement.node.node_id,
        );

        placement
            .node
            .put(&object_id, &tek, shared_bytes.clone())
            .await
            .with_context(|| format!("{}.{:?}", placement.node.node_id, placement.role))?;

        state
            .db
            .upsert_placement(
                &object_id,
                &placement.node.node_id,
                &object_id,
                &checksum,
                idx as i32,
            )
            .await?;
    }

    Ok(StatusCode::CREATED)
}

async fn get_object(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    maybe_auth: Option<BearerClaims>,
) -> Result<Response, AntArchiveError> {
    validate_key(&key)?;

    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::BucketNotFound(bucket_id.clone()))?;

    match bucket.read_policy.as_str() {
        "public" => {}
        "internal" => {
            if maybe_auth.is_none() {
                return Err(AntArchiveError::BucketNotFound(bucket_id.clone()));
            }
        }
        "private" => {
            let not_found = || AntArchiveError::ObjectNotFound(key.clone());
            let auth = maybe_auth.ok_or_else(&not_found)?;
            if bucket.client_id != auth.client_id {
                return Err(not_found());
            }
        }
        _ => {
            return Err(AntArchiveError::InternalServerError(
                "ANT-ERR-107",
                Some(anyhow::anyhow!("unknown read policy")),
            ))
        }
    }

    let object = state
        .db
        .get_object(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::ObjectNotFound(key))?;

    let placements = state.db.get_placements(&object.object_id).await?;
    if placements.is_empty() {
        return Err(AntArchiveError::InternalServerError(
            "ANT-ERR-108",
            Some(anyhow::anyhow!("no placements for object")),
        ));
    }

    let storage_nodes = resolve_storage_nodes(&state).await?;

    let mut stored_bytes_opt: Option<Vec<u8>> = None;
    for (idx, placement) in placements.iter().enumerate() {
        let Some(storage_node) = storage_nodes
            .iter()
            .find(|n| n.node_id == placement.storage_node_id)
        else {
            continue;
        };

        info!("Reading object: idx={idx} node={}", storage_node.node_id);
        let Some(bytes) = storage_node.get(&placement.storage_key).await? else {
            error!(
                node_id = %placement.storage_node_id,
                storage_key = %placement.storage_key,
                "ANT-ERR-002: blob missing from storage node: placement record exists but data does not"
            );
            continue;
        };

        if compute_checksum(&bytes) != placement.object_checksum {
            error!(
                node_id = %placement.storage_node_id,
                storage_key = %placement.storage_key,
                expected_checksum = %placement.object_checksum,
                actual_checksum = %compute_checksum(&bytes),
                "ANT-ERR-003: blob checksum mismatch: storage node returned corrupt or wrong data"
            );
            continue;
        }
        stored_bytes_opt = Some(bytes);
        break;
    }

    let stored_bytes = stored_bytes_opt.ok_or_else(|| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-109",
            Some(anyhow::anyhow!("object not readable from any placement")),
        )
    })?;

    let tek_master = load_tek_master()?;
    let maybe_tek = object
        .tek_derivation_key
        .as_deref()
        .map(|k| derive_tek(&tek_master, k))
        .transpose()?;

    let plaintext = decrypt_object(
        &load_kek(&object.kek_id, object.kek_alias.as_deref())?,
        maybe_tek.as_ref(),
        &object.encrypted_dek,
        &object.dek_nonce,
        &stored_bytes,
    )?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, plaintext.len())
        .body(Body::from(plaintext))
        .context("failed to build response")?)
}

async fn delete_object(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    auth: BearerClaims,
) -> Result<impl IntoResponse, AntArchiveError> {
    validate_key(&key)?;

    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::BucketNotFound(bucket_id.clone()))?;

    if bucket.client_id != auth.client_id {
        return Err(AntArchiveError::BucketNotFound(bucket_id.clone()));
    }

    let object = state
        .db
        .get_object(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::ObjectNotFound(key.clone()))?;

    let storage_nodes = resolve_storage_nodes(&state).await?;
    let placements = state.db.get_placements(&object.object_id).await?;
    for placement in &placements {
        let storage_node = storage_nodes
            .iter()
            .find(|n| n.node_id == placement.storage_node_id)
            .ok_or_else(|| {
                AntArchiveError::InternalServerError(
                    "ANT-ERR-110",
                    Some(anyhow::anyhow!(
                        "storage node '{}' for placement is unreachable",
                        placement.storage_node_id
                    )),
                )
            })?;
        storage_node.delete(&placement.storage_key).await?;
    }

    state.db.soft_delete_object(&bucket_id, &key).await?;

    Ok(StatusCode::OK)
}

pub fn make_routes(state: AntArchiveState) -> Router {
    use ant_library::routes::Routes;

    Routes::new()
        .put("/{bucket_id}/{*key}", put(put_object))
        .get("/{bucket_id}/{*key}", get(get_object))
        .delete("/{bucket_id}/{*key}", delete(delete_object))
        .build()
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                )))
                .layer(DefaultBodyLimit::disable())
                .layer(RequestBodyLimitLayer::new(1024 * 1024 * 1024)),
        )
}
