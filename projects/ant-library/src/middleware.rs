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
        error!("ANT-ERR-039: panic ({}): \"{}\"", s.len(), s);
    }
    // Otherwise, try to downcast to a type that implements Debug and print it
    else if let Some(debug_value) = err.downcast_ref::<&dyn std::fmt::Debug>() {
        error!("ANT-ERR-040: panic: {:?}", debug_value);
    }
    // If no specific handling, just indicate the type is unknown
    else {
        error!("ANT-ERR-041: panic: {:?}", err.type_id());
    }

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::from(
            "Internal server error, please retry.".to_string(),
        ))
        .unwrap()
}

const MAX_BODY_LOG_BYTES: usize = 64 * 1024;

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

    if bytes.len() > 0 && bytes.len() < 1024 {
        if let Ok(body) = std::str::from_utf8(&bytes) {
            tracing::debug!("{} body = {:?}", direction, body)
        }
    }

    Ok(bytes)
}

/// Axum middleware for printing requests and responses.
/// Routes with sensitive bodies (credentials, PII) should apply the
/// `redaction()` middleware before this layer.
pub async fn print_request_response(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let too_large = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
        > MAX_BODY_LOG_BYTES;

    if too_large || req.extensions().get::<Redaction>().is_some() {
        return Ok(next.run(req).await);
    }

    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let request = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(request).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body).await?;
    let response = Response::from_parts(parts, Body::from(bytes));

    Ok(response)
}

// Marker type inserted into request extensions by `redaction()`.
// `print_request_response` skips buffering when it finds this extension.
#[derive(Clone)]
struct Redaction;

/// Marks a route group's bodies as redacted so `print_request_response` skips logging them.
///
/// **Layer ordering is critical.** `redaction()` must be the *outermost* layer (added last)
/// so it runs before `print_request_response` and inserts the extension in time:
///
/// ```rust,ignore
/// Routes::new()
///     .post("/login", post(login))
///     .layer(print_request_response)  // inner — runs second, sees the extension
///     .layer(redaction())             // outer — runs first, inserts the extension
/// ```
pub async fn redaction(mut req: Request<Body>, next: Next) -> Response {
    req.extensions_mut().insert(Redaction);
    next.run(req).await
}
