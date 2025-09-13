use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::StatusCode,
    response::IntoResponse,
    routing::put,
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use base64ct::{Base64, Encoding};
use http::{header, Method};
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf};
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, limit::RequestBodyLimitLayer,
    services::ServeDir, trace::TraceLayer,
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

async fn download(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(root): State<String>,
    req: Request,
) -> Result<impl IntoResponse, StatusCode> {
    bearer_authorization(&auth)?;

    let service = ServeDir::new(PathBuf::from(root));

    return Ok(service.oneshot(req).await);

    // let path = fs_path(&path)?;
    // info!("Downloading from {}...", path.display());

    // let mut file = std::fs::File::open(path).map_err(|e: std::io::Error| {
    //     error!("{:?}", e);

    //     match e.kind() {
    //         ErrorKind::NotFound => StatusCode::NOT_FOUND,
    //         _ => StatusCode::BAD_REQUEST,
    //     }
    // })?;

    // let mut buf = Vec::new();
    // file.read_to_end(&mut buf).unwrap();

    // debug!("Read {} bytes.", buf.len());

    // return Ok((StatusCode::OK, Bytes::from(buf)));
}

async fn upload(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(root): State<String>,
    Path(path): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, StatusCode> {
    bearer_authorization(&auth)?;

    let path = PathBuf::from(root).join(path);
    info!("Uploading {}...", path.display());

    let mut file = std::fs::File::create(path).map_err(|err| {
        error!("Failed to write file: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    file.write_all(&body).unwrap();

    return Ok(StatusCode::OK);
}

pub fn make_routes(root: String) -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Router::new()
        .route("/{path}", put(upload).post(upload).get(download))
        .with_state(root)
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
