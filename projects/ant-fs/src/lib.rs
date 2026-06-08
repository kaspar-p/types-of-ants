use ant_library::routes::Routes;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use base64ct::{Base64, Encoding};
use futures::StreamExt;
use http::{header, Method};
use sha2::{Digest, Sha256};
use std::{io::ErrorKind, path::PathBuf};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, limit::RequestBodyLimitLayer};
use tracing::{debug, error, info};

fn bearer_authorization(auth: &Authorization<Basic>) -> Result<(), (StatusCode, String)> {
    let tokens = ant_library::secret::load_secret("ant_fs_users").map_err(|e| {
        error!("Failed to read authorized users: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error, please retry.".to_string(),
        )
    })?;

    if !tokens.trim().split("\n").filter(|&t| t != "").any(|t| {
        let segments: Vec<&str> = t.split(":").collect();
        let user = segments[0];
        let pass = segments[1];

        let pass_attempt_hash = Sha256::digest(&auth.0.password());
        let pass_attempt = Base64::encode_string(&pass_attempt_hash);

        return user == auth.0.username() && pass == pass_attempt;
    }) {
        return Err((StatusCode::UNAUTHORIZED, "Access denied.".to_string()));
    }

    Ok(())
}

/// Resolve a request path to its on-disk location: `<root>/<username>/<hash>`,
/// where `<hash>` is the hex SHA-256 of the request path. Hashing makes the
/// stored filename a fixed-format hex string, so arbitrary request paths
/// (slashes, `..`, special characters) can never escape the user's namespace
/// or collide with the directory structure.
fn user_file_path(root: &PathBuf, username: &str, path: &str) -> PathBuf {
    let digest = Sha256::digest(path.as_bytes());
    let hashed = base16ct::lower::encode_string(&digest);
    PathBuf::from(root).join(username).join(hashed)
}

async fn download(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(root): State<PathBuf>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    bearer_authorization(&auth)?;

    let full_path = user_file_path(&root, auth.0.username(), &path);

    let file = tokio::fs::File::open(&full_path).await.map_err(|err| match err.kind() {
        ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            format!("error: {} does not exist.\n", &path),
        ),
        _ => {
            error!("Failed to open file: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error, please retry.".to_string(),
            )
        }
    })?;

    // Stream the file from disk rather than buffering it into memory.
    Ok(Body::from_stream(ReaderStream::new(file)))
}

async fn upload(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(root): State<PathBuf>,
    Path(path): Path<String>,
    body: Body,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    bearer_authorization(&auth)?;

    let full_path = user_file_path(&root, auth.0.username(), &path);
    info!("Uploading {}...", &path);

    if let Err(e) = stream_body_to_file(&full_path, body).await {
        // Don't leave a partially-written file behind on failure.
        let _ = tokio::fs::remove_file(&full_path).await;
        return Err(e);
    }

    Ok(StatusCode::OK)
}

/// Stream a request body to disk chunk-by-chunk so memory stays O(chunk_size)
/// rather than O(file_size). Creates the user's namespace directory on demand.
async fn stream_body_to_file(full_path: &PathBuf, body: Body) -> Result<(), (StatusCode, String)> {
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            error!("Failed to create directories: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error, please retry.".to_string(),
            )
        })?;
    }

    let mut file = tokio::fs::File::create(full_path).await.map_err(|err| {
        error!("Failed to create file: {err}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error, please retry.".to_string(),
        )
    })?;

    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| {
            error!("Failed to read request body: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read request body.".to_string(),
            )
        })?;
        file.write_all(&chunk).await.map_err(|err| {
            error!("Failed to write file: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error, please retry.".to_string(),
            )
        })?;
    }

    file.flush().await.map_err(|err| {
        error!("Failed to flush file: {err}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error, please retry.".to_string(),
        )
    })?;

    Ok(())
}

async fn delete(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(root): State<PathBuf>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    bearer_authorization(&auth)?;

    let full_path = user_file_path(&root, auth.0.username(), &path);
    info!("Deleting {}...", &path);

    tokio::fs::remove_file(&full_path).await.map_err(|err| {
        error!("Failed to delete file: {err}");
        match err.kind() {
            ErrorKind::NotFound => (
                StatusCode::NOT_FOUND,
                format!("error: {} does not exist.\n", &path),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error, please retry.\n"),
            ),
        }
    })?;

    return Ok(StatusCode::OK.into_response());
}

pub fn make_routes(root: PathBuf) -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Routes::new()
        .put("/{*path}", put(upload))
        .post("/{*path}", post(upload))
        .get("/{*path}", get(download))
        .delete("/{*path}", axum::routing::delete(delete))
        .build()
        .with_state(root)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(cors)
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                )))
                .layer(DefaultBodyLimit::disable())
                .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024)),
        );

    return Ok(app);
}
