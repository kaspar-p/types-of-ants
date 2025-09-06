use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use base64ct::{Base64, Encoding};
use http::{header, Method};
use sha2::{Digest, Sha256};
use std::{
    io::{ErrorKind, Read, Write},
    path::PathBuf,
};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer,
};
use tracing::{debug, error, info};

fn bearer_authorization(auth: &Authorization<Basic>) -> Result<(), StatusCode> {
    let tokens = ant_library::secret::load_secret("ant_fs_users").map_err(|e| {
        error!("Failed to read authorized users: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !tokens.trim().split("\n").filter(|&t| t != "").any(|t| {
        let segments: Vec<&str> = t.split(":").collect();
        let user = segments[0];
        let pass = segments[1];

        let pass_attempt_hash = Sha256::digest(&auth.0.password());
        let pass_attempt = Base64::encode_string(&pass_attempt_hash);

        return user == auth.0.username() && pass == pass_attempt;
    }) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(())
}

fn fs_path(path: &str) -> Result<PathBuf, StatusCode> {
    let root_dir = dotenv::var("ANT_FS_ROOT_DIR").map_err(|e| {
        error!("No ANT_FS_ROOT_DIR variable: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(PathBuf::from(root_dir).join(path))
}

async fn download(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("Downloading {path}...");
    bearer_authorization(&auth)?;

    let mut file = std::fs::File::open(fs_path(&path)?).map_err(|e: std::io::Error| {
        error!("{:?}", e);

        match e.kind() {
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_REQUEST,
        }
    })?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();

    return Ok((StatusCode::OK, Bytes::from(buf)));
}

async fn upload(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    Path(path): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, StatusCode> {
    info!("Uploading {path}...");
    bearer_authorization(&auth)?;

    let mut file = std::fs::File::create(fs_path(&path)?).map_err(|err| {
        error!("Failed to write file: {err}");
        StatusCode::BAD_REQUEST
    })?;

    file.write_all(&body).unwrap();

    return Ok(StatusCode::OK);
}

pub fn make_routes() -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Router::new()
        .route("/{path}", get(download).put(upload).post(upload))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(CatchPanicLayer::custom(ant_library::middleware_catch_panic))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware_print_request_response,
                )))
                .layer(DefaultBodyLimit::disable())
                .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024)),
        );

    return Ok(app);
}
