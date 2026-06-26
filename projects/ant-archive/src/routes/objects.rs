use std::collections::HashMap;

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

use crate::{
    auth::BearerClaims, err::AntArchiveError, state::AntArchiveState,
    storage_client::AntArchiveStorageNodeClient,
};

/// Expects a file that's newline delimited lines that look like:
///     {hostname}:{username}:{password}
/// where each are templated, for example:
///     myhost:user1:pass1
///
/// Returns a hashmap mapping from hostname to (username, password)
fn get_client_credentials() -> Result<HashMap<String, (String, String)>, anyhow::Error> {
    let content = ant_library::secret::load_secret("ant_archive_storage_client_auths")?;

    let mut map = HashMap::new();
    for (i, line) in content.split("\n").enumerate() {
        let mut line_content = line.split(":");

        let hostname = line_content
            .next()
            .ok_or(anyhow::Error::msg(format!("Line {i} had no hostname")))?;
        let username = line_content
            .next()
            .ok_or(anyhow::Error::msg(format!("Line {i} had no username")))?;
        let password = line_content
            .next()
            .ok_or(anyhow::Error::msg(format!("Line {i} had no password")))?;

        map.insert(
            hostname.to_string(),
            (username.to_string(), password.to_string()),
        );
    }

    Ok(map)
}

async fn resolve_storage_nodes(
    state: &AntArchiveState,
) -> Result<Vec<AntArchiveStorageNodeClient>, AntArchiveError> {
    let creds = get_client_credentials()?;
    let endpoints = state.sd.resolve_all("ant-archive-storage").await;

    let mut clients = Vec::new();
    for ep in &endpoints {
        let (username, password) = creds.get(&ep.node).ok_or(anyhow::Error::msg(format!(
            "No credentials for node: {}",
            ep.node
        )))?;
        if let Some(node_id) = state.db.get_storage_node_by_node_name(&ep.node).await? {
            clients.push(AntArchiveStorageNodeClient::new(
                node_id,
                format!("http://{}:{}", ep.address, ep.port),
                username,
                password,
            ));
        }
    }

    Ok(clients)
}

fn load_kek(kek_id: &str) -> Result<[u8; 32], AntArchiveError> {
    // ant_archive_kek contains one entry per line: "{kek_id}:{base64(32 bytes)}"
    let content = ant_library::secret::load_secret("ant_archive_kek")?;
    for line in content.lines() {
        let Some((id, b64)) = line.split_once(':') else {
            continue;
        };
        if id != kek_id {
            continue;
        }
        let bytes = Base64::decode_vec(b64).map_err(|e| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "ant_archive_kek entry for '{kek_id}' is not valid base64: {e}"
            )))
        })?;
        let len = bytes.len();
        return bytes.try_into().map_err(|_| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "ant_archive_kek entry for '{kek_id}' must be exactly 32 bytes, got {len}"
            )))
        });
    }
    Err(AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
        "ant_archive_kek has no entry for kek_id '{kek_id}'"
    ))))
}

fn load_tek_master() -> Result<[u8; 32], AntArchiveError> {
    let bytes = ant_library::secret::load_secret_binary("ant_archive_tek")?;
    let len = bytes.len();
    bytes.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
            "tek must be exactly 32 bytes, got {len}"
        )))
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
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("TEK derivation failed: {e}")))
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
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "object encryption failed: {e}"
            )))
        })?;

    let mut inner = object_nonce.to_vec();
    inner.extend_from_slice(&ciphertext);

    let dek_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let encrypted_dek = kek_cipher.encrypt(&dek_nonce, dek.as_ref()).map_err(|e| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("DEK encryption failed: {e}")))
    })?;

    let tek_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let tek_key = Key::<Aes256Gcm>::from_slice(tek);
    let tek_cipher = Aes256Gcm::new(tek_key);
    let outer_ciphertext = tek_cipher
        .encrypt(&tek_nonce, inner.as_ref())
        .map_err(|e| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "TEK encryption failed: {e}"
            )))
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
                return Err(AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                    "stored bytes too short to contain TEK nonce: {} bytes",
                    stored_bytes.len()
                ))));
            }
            let (tek_nonce_bytes, outer_ciphertext) = stored_bytes.split_at(12);
            let tek_key = Key::<Aes256Gcm>::from_slice(tek);
            let tek_cipher = Aes256Gcm::new(tek_key);
            let tek_nonce = Nonce::from_slice(tek_nonce_bytes);
            tek_cipher
                .decrypt(tek_nonce, outer_ciphertext)
                .map_err(|e| {
                    AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                        "TEK decryption failed: {e}"
                    )))
                })?
        }
        None => stored_bytes.to_vec(),
    };

    if inner.len() < 12 {
        return Err(AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
            "inner blob too short to contain nonce: {} bytes",
            inner.len()
        ))));
    }
    let (object_nonce_bytes, ciphertext) = inner.split_at(12);

    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let dek_nonce = Nonce::from_slice(dek_nonce_bytes);
    let dek = kek_cipher.decrypt(dek_nonce, encrypted_dek).map_err(|e| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("DEK decryption failed: {e}")))
    })?;

    let dek_len = dek.len();
    let dek_arr: [u8; 32] = dek.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
            "DEK wrong length: expected 32 bytes, got {dek_len}"
        )))
    })?;
    let dek_key = Key::<Aes256Gcm>::from_slice(&dek_arr);
    let object_cipher = Aes256Gcm::new(dek_key);
    let object_nonce = Nonce::from_slice(object_nonce_bytes);

    object_cipher
        .decrypt(object_nonce, ciphertext)
        .map_err(|e| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "object decryption failed: {e}"
            )))
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

    let kek_id = state.db.get_active_kek_id().await?.ok_or_else(|| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("no active KEK version")))
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
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "stored tek_derivation_key has unexpected length"
            )))
        })?,
        None => generate_tek_derivation_key(&*state.rng),
    };

    let tek = derive_tek(&tek_master, &tek_derivation_key)?;
    let (encrypted_dek, dek_nonce, stored_bytes) =
        encrypt_object(&load_kek(&kek_id)?, &tek, &plaintext)?;
    let checksum = compute_checksum(&stored_bytes);

    let storage_nodes = resolve_storage_nodes(&state).await?;
    let storage_node = storage_nodes.first().ok_or_else(|| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("no active storage nodes found")))
    })?;

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

    storage_node.put(&object_id, &tek, stored_bytes).await?;

    state
        .db
        .upsert_placement(&object_id, &storage_node.node_id, &object_id, &checksum)
        .await?;

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
            return Err(AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "unknown read policy"
            ))))
        }
    }

    let object = state
        .db
        .get_object(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::ObjectNotFound(key))?;

    let placements = state.db.get_placements(&object.object_id).await?;
    let placement = placements.first().ok_or_else(|| {
        AntArchiveError::InternalServerError(Some(anyhow::anyhow!("no placements for object")))
    })?;

    let storage_nodes = resolve_storage_nodes(&state).await?;
    let storage_node = storage_nodes
        .iter()
        .find(|n| n.node_id == placement.storage_node_id)
        .ok_or_else(|| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "storage node '{}' not found in Consul",
                placement.storage_node_id
            )))
        })?;

    let stored_bytes = storage_node
        .get(&placement.storage_key)
        .await?
        .ok_or_else(|| {
            AntArchiveError::InternalServerError(Some(anyhow::anyhow!(
                "object '{}' missing from storage node '{}'",
                placement.storage_key,
                placement.storage_node_id
            )))
        })?;

    let tek_master = load_tek_master()?;
    let maybe_tek = object
        .tek_derivation_key
        .as_deref()
        .map(|k| derive_tek(&tek_master, k))
        .transpose()?;

    let plaintext = decrypt_object(
        &load_kek(&object.kek_id)?,
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
        if let Some(storage_node) = storage_nodes
            .iter()
            .find(|n| n.node_id == placement.storage_node_id)
        {
            storage_node.delete(&placement.storage_key).await?;
        }
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
