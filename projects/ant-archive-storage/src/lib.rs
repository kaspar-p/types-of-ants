pub mod err;
pub mod state;

use anyhow::Context;
use ant_library::routes::Routes;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{delete, get, head, put},
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use axum_prometheus::{PrometheusMetricLayer, PrometheusMetricLayerBuilder};
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::OnceLock;
use base64ct::{Base64, Encoding};
use futures::StreamExt;
use http::{header, StatusCode};
use sha2::{Digest, Sha256};
use std::{
    io::ErrorKind,
    path::Path as FsPath,
    path::PathBuf,
    sync::atomic::Ordering,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer};
use tracing::{error, info};

pub use err::AntArchiveStorageError;
pub use state::AntArchiveStorageState;

/// On-disk encoding version byte written at the start of every blob file.
const ENCODING_V1: u8 = 1;

/// Compute the sharded blob path for a given storage key:
/// `{root}/blobs/{h[0..2]}/{h[2..4]}/{h}` where h = hex(sha256(key)).
pub fn blob_path(root: &FsPath, storage_key: &str) -> PathBuf {
    let digest = Sha256::digest(storage_key.as_bytes());
    let h = base16ct::lower::encode_string(&digest);
    root.join("blobs").join(&h[0..2]).join(&h[2..4]).join(&h)
}

fn authenticate(auth: &Authorization<Basic>) -> Result<(), AntArchiveStorageError> {
    let tokens = ant_library::secret::load_secret("archive_storage_auth")
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

            // Constant-time comparison for the password hash to avoid
            // timing leaks on the credential check.
            stored_user == auth.0.username()
                && constant_time_eq(stored_hash.as_bytes(), attempt_b64.as_bytes())
        });

    if !authorized {
        return Err(AntArchiveStorageError::AccessDenied);
    }

    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Parse an HTTP Range header value (bytes=start-end, bytes=start-, bytes=-suffix)
/// against a known logical size. Returns (start, end) inclusive, or None if the
/// range is invalid or unsatisfiable.
fn parse_range(range_header: &str, logical_size: u64) -> Option<(u64, u64)> {
    let s = range_header.strip_prefix("bytes=")?;

    if let Some(suffix) = s.strip_prefix('-') {
        let n: u64 = suffix.parse().ok()?;
        if n == 0 {
            return None;
        }
        let start = logical_size.saturating_sub(n);
        return Some((start, logical_size - 1));
    }

    let (start_str, end_str) = s.split_once('-')?;
    let start: u64 = start_str.parse().ok()?;
    let end = if end_str.is_empty() {
        logical_size.saturating_sub(1)
    } else {
        end_str.parse().ok()?
    };

    if logical_size == 0 || start > end || end >= logical_size {
        return None;
    }
    Some((start, end))
}

async fn put_blob(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
    body: Body,
) -> Result<impl IntoResponse, AntArchiveStorageError> {
    authenticate(&auth)?;

    let dest = blob_path(&state.root, &storage_key);
    info!("PUT blob: {storage_key}");

    // Capture old logical size before overwriting, if the blob already exists.
    let old_logical_size = tokio::fs::metadata(&dest)
        .await
        .ok()
        .map(|m| m.len().saturating_sub(1));

    let tmp_dir = state.root.join("tmp");
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .context("Failed to create tmp dir")?;

    let tmp_path = tmp_dir.join(uuid::Uuid::new_v4().to_string());

    if let Err(e) = stream_body_to_tmp(&tmp_path, body).await {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(e);
    }

    let new_logical_size = tokio::fs::metadata(&tmp_path)
        .await
        .context("Failed to stat tmp file")?
        .len()
        .saturating_sub(1);

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create blob dir")?;
    }

    tokio::fs::rename(&tmp_path, &dest)
        .await
        .context("Failed to rename tmp blob")?;

    if let Some(old) = old_logical_size {
        state.adjust_bytes(-(old as i64));
    }
    state.adjust_bytes(new_logical_size as i64);
    Ok(StatusCode::CREATED)
}

/// Write ENCODING_V1 prefix + streamed body to a tmp file, then fsync.
async fn stream_body_to_tmp(
    tmp_path: &FsPath,
    body: Body,
) -> Result<(), AntArchiveStorageError> {
    let mut file = tokio::fs::File::create(tmp_path)
        .await
        .context("Failed to create tmp file")?;

    file.write_all(&[ENCODING_V1])
        .await
        .context("Failed to write encoding byte")?;

    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read request body")?;
        file.write_all(&chunk).await.context("Failed to write blob chunk")?;
    }

    file.flush().await.context("Failed to flush blob")?;
    file.sync_all().await.context("Failed to fsync blob")?;

    Ok(())
}

async fn get_blob(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AntArchiveStorageError> {
    authenticate(&auth)?;

    let path = blob_path(&state.root, &storage_key);

    let mut file = tokio::fs::File::open(&path).await.map_err(|e| match e.kind() {
        ErrorKind::NotFound => AntArchiveStorageError::NotFound(storage_key.clone()),
        _ => AntArchiveStorageError::InternalServerError(Some(e.into())),
    })?;

    let metadata = file.metadata().await.context("Failed to stat blob")?;

    let physical_size = metadata.len();
    if physical_size == 0 {
        error!("Blob {storage_key} is empty (no encoding byte)");
        return Err(AntArchiveStorageError::InternalServerError(None));
    }

    let mut version_buf = [0u8; 1];
    file.read_exact(&mut version_buf)
        .await
        .context("Failed to read encoding byte")?;

    if version_buf[0] != ENCODING_V1 {
        error!("Unknown encoding version {} for {storage_key}", version_buf[0]);
        return Err(AntArchiveStorageError::InternalServerError(None));
    }

    let logical_size = physical_size - 1;

    if let Some(range_val) = headers.get(header::RANGE) {
        let range_str = range_val.to_str().unwrap_or("");
        let Some((start, end)) = parse_range(range_str, logical_size) else {
            return Err(AntArchiveStorageError::RangeNotSatisfiable);
        };

        // Seek to physical position: skip version byte + skip to logical start.
        let physical_start = 1 + start;
        file.seek(std::io::SeekFrom::Start(physical_start))
            .await
            .context("Failed to seek blob")?;

        let range_len = end - start + 1;
        let content_range = format!("bytes {start}-{end}/{logical_size}");

        return Ok(Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_LENGTH, range_len)
            .header(header::CONTENT_RANGE, content_range)
            .body(Body::from_stream(ReaderStream::new(file.take(range_len))))
            .context("Failed to build range response")?);
    }

    // Full content — file cursor is already past the version byte.
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, logical_size)
        .body(Body::from_stream(ReaderStream::new(file)))
        .context("Failed to build response")?)
}

async fn head_blob(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
) -> Result<Response, AntArchiveStorageError> {
    authenticate(&auth)?;

    let path = blob_path(&state.root, &storage_key);

    let metadata = tokio::fs::metadata(&path).await.map_err(|e| match e.kind() {
        ErrorKind::NotFound => AntArchiveStorageError::NotFound(storage_key.clone()),
        _ => AntArchiveStorageError::InternalServerError(Some(e.into())),
    })?;

    let physical_size = metadata.len();
    let logical_size = physical_size.saturating_sub(1);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, logical_size)
        .body(Body::empty())
        .context("Failed to build head response")?)
}

async fn delete_blob(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<AntArchiveStorageState>,
    Path(storage_key): Path<String>,
) -> Result<impl IntoResponse, AntArchiveStorageError> {
    authenticate(&auth)?;

    let path = blob_path(&state.root, &storage_key);
    info!("DELETE blob: {storage_key}");

    let logical_size = tokio::fs::metadata(&path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => AntArchiveStorageError::NotFound(storage_key.clone()),
            _ => AntArchiveStorageError::InternalServerError(Some(e.into())),
        })?
        .len()
        .saturating_sub(1);

    tokio::fs::remove_file(&path).await.map_err(|e| match e.kind() {
        ErrorKind::NotFound => AntArchiveStorageError::NotFound(storage_key.clone()),
        _ => AntArchiveStorageError::InternalServerError(Some(e.into())),
    })?;

    state.adjust_bytes(-(logical_size as i64));
    Ok(StatusCode::OK)
}

/// Sweep the tmp directory on startup to remove any torn uploads from a
/// previous crash, then recreate it and write the LAYOUT_VERSION marker.
pub async fn startup_init(root: &PathBuf) -> Result<(), anyhow::Error> {
    let tmp_dir = root.join("tmp");
    if tmp_dir.exists() {
        tokio::fs::remove_dir_all(&tmp_dir).await?;
    }
    tokio::fs::create_dir_all(&tmp_dir).await?;
    tokio::fs::create_dir_all(root.join("blobs")).await?;
    tokio::fs::write(root.join("LAYOUT_VERSION"), "1\n").await?;
    Ok(())
}

async fn metrics_handler(State(state): State<AntArchiveStorageState>) -> (StatusCode, String) {
    let mut output = state.metrics_handle.render();
    let bytes = state.bytes_stored.load(Ordering::Relaxed);
    output.push_str(&format!(
        "# HELP ant_archive_storage_bytes_stored Total logical bytes currently stored\n\
         # TYPE ant_archive_storage_bytes_stored gauge\n\
         ant_archive_storage_bytes_stored {bytes}\n"
    ));
    (StatusCode::OK, output)
}

pub fn make_metrics_routes(state: AntArchiveStorageState) -> Router {
    Router::new().route("/metrics", get(metrics_handler)).with_state(state)
}

/// axum-prometheus installs a global `metrics` recorder and sets global metric-name strings,
/// both of which can only happen once per process. `get_or_init` ensures a single
/// initialization even under parallel test execution; every subsequent call gets
/// a fresh layer pointing at the same underlying global recorder.
static GLOBAL_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn build_metric_layer() -> (PrometheusMetricLayer<'static>, PrometheusHandle) {
    let handle = GLOBAL_HANDLE.get_or_init(|| {
        let (_, h) = PrometheusMetricLayerBuilder::new()
            .with_prefix("ant_archive_storage")
            .with_default_metrics()
            .build_pair();
        h
    });

    let (layer, _) = PrometheusMetricLayerBuilder::new()
        .with_metrics_from_fn(|| handle.clone())
        .build_pair();
    (layer, handle.clone())
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
        .with_state(state.clone())
        .layer(
            ServiceBuilder::new()
                .layer(metric_layer)
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
