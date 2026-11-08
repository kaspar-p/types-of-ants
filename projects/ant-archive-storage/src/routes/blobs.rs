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
    extract::{DefaultBodyLimit, FromRequestParts, Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{delete, get, head, put},
    Json, Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use axum_prometheus::PrometheusMetricLayer;
use base64ct::{Base64, Encoding};
use chrono::{DateTime, Utc};
use http::request::Parts;
use http::{header, StatusCode};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::ErrorKind;
use std::{path::Path as FsPath, path::PathBuf};
use subtle::ConstantTimeEq;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer};
use tracing::info;

/// A blob on disk. The storage key isn't stored, only its hash, so callers that know the key
/// can match it with `sha256(storage_key)`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Blob {
    pub hash: String,
    /// Logical size, as returned by HEAD.
    pub size: u64,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListBlobsResponse {
    /// Sorted by hash.
    pub blobs: Vec<Blob>,
    /// Pass as `after` to get the next page. None on the last page.
    pub next: Option<String>,
}

/// Compute the sharded blob path for a given storage key:
/// `{root}/blobs/{h[0..2]}/{h[2..4]}/{h}` where h = hex(sha256(key)).
pub fn blob_path(root: &FsPath, storage_key: &str) -> PathBuf {
    let digest = Sha256::digest(storage_key.as_bytes());
    let h = base16ct::lower::encode_string(&digest);
    hash_path(root, &h)
}

/// `hash` must satisfy `is_blob_hash`, so it can't escape the blobs directory.
fn hash_path(root: &FsPath, hash: &str) -> PathBuf {
    root.join("blobs")
        .join(&hash[0..2])
        .join(&hash[2..4])
        .join(hash)
}

fn is_hex(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_blob_hash(s: &str) -> bool {
    s.len() == 64 && is_hex(s)
}

/// Names in `dir` that satisfy `keep`, sorted. A missing directory has no names.
async fn sorted_names(
    dir: &FsPath,
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<String>, AntArchiveStorageError> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => {
            return Err(AntArchiveStorageError::InternalServerError(
                "ANT-ERR-141",
                Some(e.into()),
            ));
        }
    };

    let mut names = vec![];
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| AntArchiveStorageError::InternalServerError("ANT-ERR-142", Some(e.into())))?
    {
        if let Some(name) = entry.file_name().to_str() {
            if keep(name) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
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

async fn tmp_dir(state: &AntArchiveStorageState) -> Result<PathBuf, tokio::io::Error> {
    let dir = state.root.join("_tmpdir");
    tokio::fs::create_dir_all(&dir).await?;
    return Ok(dir);
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

    let tmp_dir = tmp_dir(&state).await.context("failed to create tmp dir")?;
    let tmp = tempfile::NamedTempFile::new_in(&tmp_dir).context("Failed to create tmp file")?;
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

    info!("DELETE blob: {storage_key}");
    remove_blob(&state, &blob_path(&state.root, &storage_key), &storage_key).await
}

/// For blobs whose storage key the caller doesn't know, e.g. ones found by `list_blobs` that no
/// placement points to.
async fn delete_blob_by_hash(
    BasicAuth(auth): BasicAuth,
    State(state): State<AntArchiveStorageState>,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, AntArchiveStorageError> {
    authenticate(&auth)?;

    if !is_blob_hash(&hash) {
        return Err(AntArchiveStorageError::BadRequest(
            "hash must be 64 lowercase hex characters".to_string(),
        ));
    }

    info!("DELETE blob by hash: {hash}");
    remove_blob(&state, &hash_path(&state.root, &hash), &hash).await
}

async fn remove_blob(
    state: &AntArchiveStorageState,
    path: &FsPath,
    name: &str,
) -> Result<StatusCode, AntArchiveStorageError> {
    let size = BlobHandle::size(path).await?;

    tokio::fs::remove_file(path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => AntArchiveStorageError::NotFound(name.to_string()),
            _ => AntArchiveStorageError::InternalServerError("ANT-ERR-112", Some(e.into())),
        })?;

    state.adjust_bytes(-(size as i64));
    Ok(StatusCode::OK)
}

const DEFAULT_LIST_LIMIT: usize = 1000;
const MAX_LIST_LIMIT: usize = 100_000;

#[derive(Deserialize)]
struct ListBlobsQuery {
    /// Only list blobs whose hash sorts after this one.
    after: Option<String>,
    limit: Option<usize>,
}

/// Walks the sharded blob directories in order, so pages are sorted by hash and `after` can
/// skip whole directories instead of reading them.
async fn list_blobs(
    BasicAuth(auth): BasicAuth,
    State(state): State<AntArchiveStorageState>,
    Query(query): Query<ListBlobsQuery>,
) -> Result<Json<ListBlobsResponse>, AntArchiveStorageError> {
    authenticate(&auth)?;

    let limit = query.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    if limit == 0 || limit > MAX_LIST_LIMIT {
        return Err(AntArchiveStorageError::BadRequest(format!(
            "limit must be between 1 and {MAX_LIST_LIMIT}"
        )));
    }
    if let Some(after) = &query.after {
        if !is_blob_hash(after) {
            return Err(AntArchiveStorageError::BadRequest(
                "after must be 64 lowercase hex characters".to_string(),
            ));
        }
    }
    let after = query.after.as_deref().unwrap_or("");

    let blobs_dir = state.root.join("blobs");
    let is_shard_dir = |name: &str| name.len() == 2 && is_hex(name);

    // One extra, to know whether there's another page.
    let mut blobs: Vec<Blob> = vec![];
    'walk: for h2 in sorted_names(&blobs_dir, is_shard_dir).await? {
        if h2.as_str() < &after[..after.len().min(2)] {
            continue;
        }

        for h4 in sorted_names(&blobs_dir.join(&h2), is_shard_dir).await? {
            if format!("{h2}{h4}").as_str() < &after[..after.len().min(4)] {
                continue;
            }

            let dir = blobs_dir.join(&h2).join(&h4);
            let prefix = format!("{h2}{h4}");
            let hashes =
                sorted_names(&dir, |name| is_blob_hash(name) && name.starts_with(&prefix)).await?;

            for hash in hashes {
                if hash.as_str() <= after {
                    continue;
                }

                let path = dir.join(&hash);
                // Skip blobs deleted since the directory was read.
                let size = match BlobHandle::size(&path).await {
                    Ok(size) => size,
                    Err(CodecError::NotFound(_)) => continue,
                    Err(e) => return Err(e.into()),
                };
                let modified = match tokio::fs::metadata(&path).await {
                    Ok(metadata) => metadata.modified().map_err(|e| {
                        AntArchiveStorageError::InternalServerError("ANT-ERR-143", Some(e.into()))
                    })?,
                    Err(e) if e.kind() == ErrorKind::NotFound => continue,
                    Err(e) => {
                        return Err(AntArchiveStorageError::InternalServerError(
                            "ANT-ERR-144",
                            Some(e.into()),
                        ));
                    }
                };

                blobs.push(Blob {
                    hash,
                    size,
                    modified: DateTime::<Utc>::from(modified),
                });
                if blobs.len() > limit {
                    break 'walk;
                }
            }
        }
    }

    let next = if blobs.len() > limit {
        blobs.truncate(limit);
        blobs.last().map(|b| b.hash.clone())
    } else {
        None
    };

    Ok(Json(ListBlobsResponse { blobs, next }))
}

pub fn make_routes(
    state: AntArchiveStorageState,
    metric_layer: PrometheusMetricLayer<'static>,
) -> Result<Router, anyhow::Error> {
    let app = Routes::new()
        .get("/", get(list_blobs))
        .delete("/blobs/{hash}", delete(delete_blob_by_hash))
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
