use axum::{
    extract::{DefaultBodyLimit, Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use http::{header, Method};
use std::{io::Write, path::PathBuf};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer,
};
use tracing::{debug, error};

async fn download() {}

async fn upload(
    Path(path): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    while let Some(mut field) = multipart.next_field().await.map_err(|err| {
        error!("Failed to get next field: {err}");
        StatusCode::BAD_REQUEST
    })? {
        error!("Failed to write file");

        let mut file =
            std::fs::File::create(PathBuf::from("archive").join(path)).map_err(|err| {
                error!("Failed to write file: {err}");
                StatusCode::BAD_REQUEST
            })?;
        while let Some(chunk) = field.chunk().await.map_err(|err| {
            error!("Failed to read next chunk: {err}");
            StatusCode::BAD_REQUEST
        })? {
            file.write_all(&chunk).unwrap();
        }

        return Ok(StatusCode::OK);
    }

    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn make_routes() -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        // .allow_origin(AllowOrigin::any())
        // .allow_credentials(true)
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
