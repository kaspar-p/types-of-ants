use crate::{
    codec::{BlobHandle, CodecError},
    err::AntArchiveStorageError,
    state::AntArchiveStorageState,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use ant_library::routes::Routes;
use anyhow::Context;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, FromRequestParts, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{delete, get, head, put},
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use http::request::Parts;
use axum_prometheus::PrometheusMetricLayer;
use base64ct::{Base64, Encoding};
use http::{header, StatusCode};
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use std::io::ErrorKind;
use std::{path::Path as FsPath, path::PathBuf};
use subtle::ConstantTimeEq;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer};
use tracing::info;

/// Compute the sharded blob path for a given storage key:
/// `{root}/blobs/{h[0..2]}/{h[2..4]}/{h}` where h = hex(sha256(key)).
pub fn blob_path(root: &FsPath, storage_key: &str) -> PathBuf {
    let digest = Sha256::digest(storage_key.as_bytes());
    let h = base16ct::lower::encode_string(&digest);
    root.join("blobs").join(&h[0..2]).join(&h[2..4]).join(&h)
}

struct BasicAuth(Authorization<Basic>);

impl<S: Send + Sync> FromRequestParts<S> for BasicAuth {
    type Rejection = AntArchiveStorageError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Option::<TypedHeader<Authorization<Basic>>>::from_request_parts(parts, state)
            .await
            .ok()
            .flatten()
            .map(|TypedHeader(auth)| BasicAuth(auth))
            .ok_or(AntArchiveStorageError::AccessDenied)
    }
}

fn authenticate(auth: &Authorization<Basic>) -> Result<(), AntArchiveStorageError> {
    let tokens = ant_library::secret::load_secret("ant_archive_storage_auth")
        .context("Failed to read auth secret")?;

    let attempt_hash = Sha256::digest(auth.0.password().as_bytes());
    let attempt_b64 = Base64::encode_string(&attempt_hash);

    let authorized = tokens
        .trim()
        .split('\n')
        .filter(|t| !t.is_empty())
        .any(|t| {
            let mut parts = t.splitn(2, ':');
            let stored_user = parts.next().unwrap_or("");
            let stored_hash = parts.next().unwrap_or("");

            stored_user == auth.0.username()
                && bool::from(stored_hash.as_bytes().ct_eq(attempt_b64.as_bytes()))
        });

    if !authorized {
        return Err(AntArchiveStorageError::AccessDenied);
    }

    Ok(())
}

/// Parse an HTTP Range header value (bytes=start-end, bytes=start-, bytes=-suffix)
/// against a known size. Returns (start, end) inclusive, or None if the
/// range is invalid or unsatisfiable.
fn parse_range(range_header: &str, size: u64) -> Option<(u64, u64)> {
    let s = range_header.strip_prefix("bytes=")?;

    if let Some(suffix) = s.strip_prefix('-') {
        let n: u64 = suffix.parse().ok()?;
        if n == 0 {
            return None;
        }
        let start = size.saturating_sub(n);
        return Some((start, size - 1));
    }

    let (start_str, end_str) = s.split_once('-')?;
    let start: u64 = start_str.parse().ok()?;
    let end = if end_str.is_empty() {
        size.saturating_sub(1)
    } else {
        end_str.parse().ok()?
    };

    if size == 0 || start > end || end >= size {
        return None;
    }
    Some((start, end))
}

async fn put_blob(
    BasicAuth(auth): BasicAuth,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse, AntArchiveStorageError> {
    authenticate(&auth)?;

    let tek_hex = headers
        .get("x-ant-tek")
        .ok_or_else(|| AntArchiveStorageError::BadRequest("X-Ant-Tek header missing".to_string()))?
        .to_str()
        .map_err(|_| {
            AntArchiveStorageError::BadRequest("X-Ant-Tek header is not valid UTF-8".to_string())
        })?;

    let mut tek = [0u8; 32];
    base16ct::lower::decode(tek_hex, &mut tek).map_err(|_| {
        AntArchiveStorageError::BadRequest("X-Ant-Tek header is not valid hex".to_string())
    })?;

    let outer = body
        .collect()
        .await
        .context("Failed to read request body")?
        .to_bytes()
        .to_vec();

    // Decrypt outer with TEK to validate the inner blob in memory; discard both after.
    if outer.len() < 12 {
        return Err(AntArchiveStorageError::BadRequest(
            "outer blob too short to contain TEK nonce".to_string(),
        ));
    }
    let (tek_nonce_bytes, outer_ciphertext) = outer.split_at(12);
    let tek_key = Key::<Aes256Gcm>::from_slice(&tek);
    let tek_cipher = Aes256Gcm::new(tek_key);
    let tek_nonce = Nonce::from_slice(tek_nonce_bytes);
    {
        tek_cipher
            .decrypt(tek_nonce, outer_ciphertext)
            .map_err(|_| AntArchiveStorageError::BadRequest("TEK decryption failed".to_string()))?;
    }

    let dest = blob_path(&state.root, &storage_key);
    info!("PUT blob: {storage_key}");

    let old_size = match BlobHandle::size(&dest).await {
        Ok(size) => Some(size),
        Err(CodecError::NotFound(_)) => None,
        Err(e) => return Err(e.into()),
    };

    let tmp = tempfile::NamedTempFile::new().context("Failed to create tmp file")?;
    let tmp_path = tmp.path().to_path_buf();

    // On error, tmp drops and auto-deletes the file.
    BlobHandle::write(&tmp_path, outer.as_slice()).await?;

    let new_size = BlobHandle::size(&tmp_path).await?;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create blob dir")?;
    }

    // Atomically move to final destination; TempPath auto-deletes if persist fails.
    tmp.into_temp_path()
        .persist(&dest)
        .context("Failed to persist blob")?;

    if let Some(old) = old_size {
        state.adjust_bytes(-(old as i64));
    }
    state.adjust_bytes(new_size as i64);
    Ok(StatusCode::CREATED)
}

async fn get_blob(
    BasicAuth(auth): BasicAuth,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AntArchiveStorageError> {
    authenticate(&auth)?;

    let path = blob_path(&state.root, &storage_key);

    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => AntArchiveStorageError::NotFound(storage_key.clone()),
            _ => AntArchiveStorageError::InternalServerError("ANT-ERR-111", Some(e.into())),
        })?;

    let mut handle = BlobHandle::open(file).await?;
    let size = handle.size;

    if let Some(range_val) = headers.get(header::RANGE) {
        let range_str = range_val.to_str().unwrap_or("");
        let Some((start, end)) = parse_range(range_str, size) else {
            return Err(AntArchiveStorageError::RangeNotSatisfiable);
        };

        handle.seek(start).await?;
        let range_len = end - start + 1;
        let content_range = format!("bytes {start}-{end}/{size}");

        return Ok(Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_LENGTH, range_len)
            .header(header::CONTENT_RANGE, content_range)
            .body(Body::from_stream(ReaderStream::new(handle.take(range_len))))
            .context("Failed to build range response")?);
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, size)
        .body(Body::from_stream(ReaderStream::new(handle)))
        .context("Failed to build response")?)
}

async fn head_blob(
    BasicAuth(auth): BasicAuth,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
) -> Result<Response, AntArchiveStorageError> {
    authenticate(&auth)?;

    let path = blob_path(&state.root, &storage_key);

    let size = BlobHandle::size(&path).await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, size)
        .body(Body::empty())
        .context("Failed to build head response")?)
}

async fn delete_blob(
    BasicAuth(auth): BasicAuth,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
) -> Result<impl IntoResponse, AntArchiveStorageError> {
    authenticate(&auth)?;

    let path = blob_path(&state.root, &storage_key);
    info!("DELETE blob: {storage_key}");

    let size = BlobHandle::size(&path).await?;

    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => AntArchiveStorageError::NotFound(storage_key.clone()),
            _ => AntArchiveStorageError::InternalServerError("ANT-ERR-112", Some(e.into())),
        })?;

    state.adjust_bytes(-(size as i64));
    Ok(StatusCode::OK)
}

pub fn make_routes(
    state: AntArchiveStorageState,
    metric_layer: PrometheusMetricLayer<'static>,
) -> Result<Router, anyhow::Error> {
    let app = Routes::new()
        .put("/{storage_key}", put(put_blob))
        .get("/{storage_key}", get(get_blob))
        .head("/{storage_key}", head(head_blob))
        .delete("/{storage_key}", delete(delete_blob))
        .build()
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(metric_layer)
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                )))
                .layer(DefaultBodyLimit::disable())
                .layer(RequestBodyLimitLayer::new(1024 * 1024 * 1024)),
        );

    Ok(app)
}
