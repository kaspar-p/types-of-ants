use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http::{header, HeaderValue};
use http_body_util::BodyExt;
use http_body_util::Full;
use std::{env::set_var, fmt::Display};
use tracing::{debug, error, Level};
use tracing_subscriber::{fmt::writer::Tee, FmtSubscriber};

pub mod db;
pub mod headers;
pub mod host_architecture;
pub mod manifest_file;
pub mod secret;

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
    // SAFETY: according to this discussion: https://users.rust-lang.org/t/unsafe-std-set-var-change/112704/68
    // this function is unsafe if-and-only-if there is FFI code modifying the environment lock that the
    // rust standard library sets. We don't use FFI!
    unsafe {
        set_var(
        "RUST_LOG",
        format!(
            "{}=debug,ant_library=debug,ant_data_farm=debug,glimmer=debug,tower_http=debug,axum::rejection=trace",
            project
        ),
    );
    }
    dotenv::dotenv().expect("No .env file found!");

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_ansi(false)
        .with_writer(Tee::new(
            std::io::stdout,
            tracing_appender::rolling::hourly("./logs", format!("{}.log", project)),
        ))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    debug!("Logs initialized...");
}

async fn buffer_and_print<B>(
    direction: &str,
    body: B,
    redact: bool,
) -> Result<Bytes, (StatusCode, String)>
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
        if redact {
            tracing::debug!("{} body = {{REDACTED}}", direction)
        } else {
            tracing::debug!("{} body = {:?}", direction, body)
        };
    }

    Ok(bytes)
}

/// Axum middleware for printing requests and responses.
/// Should be used as a middleware layer for all types-of-ants web servers.
/// Use `ignore_paths` to specify the paths which to REDACT the request and response.
pub async fn middleware_print_request_response(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let redact = req.uri().path().contains("/signup")
        || req.uri().path().contains("/login")
        || req.uri().path().contains("/verification-attempt");

    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body, redact).await?;
    let request = Request::from_parts(parts, Body::from(bytes));
    let res = next.run(request).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body, redact).await?;
    let response = Response::from_parts(parts, Body::from(bytes));

    Ok(response)
}

#[derive(Debug)]
pub enum Mode {
    Dev,
    Prod,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Mode::Dev => f.write_str("DEV")?,
            Mode::Prod => f.write_str("PRODUCTION")?,
        };
        Ok(())
    }
}

pub fn get_mode() -> Mode {
    match dotenv::var("ANT_ON_THE_WEB_MODE") {
        Ok(mode) if mode.as_str() == "dev" => Mode::Dev,
        _ => Mode::Prod,
    }
}

pub async fn middleware_mode_headers(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut response = next.run(req).await;

    let response = match get_mode() {
        Mode::Dev => {
            let headers = response.headers_mut();
            headers.append(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_str("true").unwrap(),
            );
            response
        }
        Mode::Prod => response,
    };
    return Ok(response);
}

/// Axum middleware for translating a panicked handler into a 500 InternalServerError.
/// The thread that panics will die, but new threads and requests will succeed.
/// All webservers should use this layer.
pub fn middleware_catch_panic(
    err: Box<dyn std::any::Any + Send + 'static>,
) -> Response<Full<Bytes>> {
    // Try to downcast to a String and print its length and content
    if let Some(s) = err.downcast_ref::<String>() {
        error!("panic ({}): \"{}\"", s.len(), s);
    }
    // Otherwise, try to downcast to a type that implements Debug and print it
    else if let Some(debug_value) = err.downcast_ref::<&dyn std::fmt::Debug>() {
        error!("panic: {:?}", debug_value);
    }
    // If no specific handling, just indicate the type is unknown
    else {
        error!("panic: {:?}", err.type_id());
    }

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::from(
            "Internal server error, please retry.".to_string(),
        ))
        .unwrap()
}
