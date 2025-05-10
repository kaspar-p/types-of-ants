use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use tracing::{debug, Level};
use tracing_subscriber::FmtSubscriber;

pub mod axum_test_client;

/// The standard ping that all typesofants web servers should use.
pub async fn api_ping() -> (StatusCode, String) {
    (StatusCode::OK, "healthy ant".to_string())
}

/// An API fallback function declaring which routes exist for the user to query.
pub fn api_fallback(routes: &[&str]) -> (StatusCode, String) {
    (
        StatusCode::NOT_FOUND,
        format!(
            "Unknown route. Valid routes are:\n{}",
            routes
                .iter()
                .map(|&r| String::from(r))
                .map(|r| { " -> ".to_owned() + &r + "\n" })
                .collect::<String>()
        ),
    )
}

pub fn set_global_logs(project: &str) -> () {
    std::env::set_var(
        "RUST_LOG",
        format!(
            "{}=debug,ant_library=debug,ant_data_farm=debug,glimmer=debug,tower_http=debug,axum::rejection=trace",
            project
        ),
    );
    dotenv::dotenv().expect("No .env file found!");

    // initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_ansi(false)
        .with_writer(tracing_appender::rolling::hourly(
            "./logs",
            format!("{}.log", project),
        ))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    debug!("Logs initialized...");
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collection) => collection.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}

/// Axum middleware for printing requests and responses.
/// Should be used as a middleware layer for all types-of-ants web servers.
pub async fn middleware_print_request_response(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let request = Request::from_parts(parts, Body::from(bytes));
    let res = next.run(request).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body).await?;
    let response = Response::from_parts(parts, Body::from(bytes));

    Ok(response)
}
