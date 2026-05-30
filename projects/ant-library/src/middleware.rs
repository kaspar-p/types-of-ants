use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http::header;
use http_body_util::{BodyExt, Full};
use tracing::error;

#[derive(Clone)]
pub struct SkipOnRequest;

impl<B> tower_http::trace::OnRequest<B> for SkipOnRequest {
    fn on_request(&mut self, _: &http::Request<B>, _: &tracing::Span) {}
}

pub fn http_log_layer() -> tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    tower_http::trace::DefaultMakeSpan,
    SkipOnRequest,
> {
    tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
        .on_request(SkipOnRequest)
        .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO))
}

/// Axum middleware for translating a panicked handler into a 500 InternalServerError.
/// The thread that panics will die, but new threads and requests will succeed.
/// All webservers should use this layer.
pub fn catch_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response<Full<Bytes>> {
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

async fn buffer_and_print<B>(
    direction: &str,
    body: B,
    redact: bool,
    ignore: bool,
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

    if !ignore && !redact && bytes.len() > 0 && bytes.len() < 1024 {
        if let Ok(body) = std::str::from_utf8(&bytes) {
            tracing::debug!("{} body = {:?}", direction, body)
        }
    }

    Ok(bytes)
}

/// Axum middleware for printing requests and responses.
/// Should be used as a middleware layer for all types-of-ants web servers.
/// Use `ignore_paths` to specify the paths which to REDACT the request and response.
pub async fn print_request_response(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let too_large_to_print = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
        > 1024;

    if too_large_to_print {
        return Ok(next.run(req).await);
    }

    let redact = req.uri().path().contains("/signup")
        || req.uri().path().contains("/login")
        || req.uri().path().contains("/verification-attempt");

    let ignore = req.uri().path().contains("/deployment/iteration");

    if req.extensions().get::<Redaction>().is_some() || redact {
        return Ok(next.run(req).await);
    }

    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body, redact, ignore).await?;
    let request = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(request).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body, redact, ignore).await?;
    let response = Response::from_parts(parts, Body::from(bytes));

    Ok(response)
}

#[derive(Clone)]
struct Redaction;

/// Mark a route's body as redacted — buffered but not logged (credentials, PII).
/// Apply before request_response_logging() in the layer stack.
pub async fn redaction(mut req: Request<Body>, next: Next) -> Response {
    req.extensions_mut().insert(Redaction);
    next.run(req).await
}
