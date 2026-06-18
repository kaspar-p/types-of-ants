use ant_library_test::axum_test_client::TestClient;
use axum::{body::Body, routing::post, Router};
use http::StatusCode;
use tracing_test::traced_test;

async fn ok_handler(body: axum::body::Bytes) -> (StatusCode, axum::body::Bytes) {
    (StatusCode::OK, body)
}

fn logged_router() -> Router {
    Router::new()
        .route("/echo", post(ok_handler))
        .layer(axum::middleware::from_fn(
            ant_library::middleware::print_request_response,
        ))
}

fn redacted_router() -> Router {
    // redaction() must be outermost (added last) so it runs before
    // print_request_response and inserts the extension in time.
    Router::new()
        .route("/secret", post(ok_handler))
        .layer(axum::middleware::from_fn(
            ant_library::middleware::print_request_response,
        ))
        .layer(axum::middleware::from_fn(ant_library::middleware::redaction))
}

#[tokio::test]
#[traced_test]
async fn print_request_response_logs_small_body() {
    let client = TestClient::new(logged_router()).await;

    {
        let res = client
            .post("/echo")
            .body(r#"{"sentinel":"xk9z"}"#)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    assert!(logs_contain("xk9z"), "small body should appear in logs");
}

#[tokio::test]
#[traced_test]
async fn print_request_response_skips_redacted_route() {
    let client = TestClient::new(redacted_router()).await;

    {
        let res = client
            .post("/secret")
            .body(r#"{"password":"hunter2"}"#)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    assert!(
        !logs_contain("hunter2"),
        "redacted body must not appear in logs"
    );
}

#[tokio::test]
#[traced_test]
async fn print_request_response_skips_body_exceeding_threshold() {
    let client = TestClient::new(logged_router()).await;

    let large_body = "z".repeat(65 * 1024);
    {
        let res = client
            .post("/echo")
            .header("content-length", &(65 * 1024).to_string())
            .body(large_body.clone())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    assert!(
        !logs_contain("zzzzzz"),
        "body exceeding 64KB threshold must not be buffered or logged"
    );
}

#[tokio::test]
#[traced_test]
async fn print_request_response_streams_large_body_to_handler() {
    let client = TestClient::new(logged_router()).await;

    let large_body = "a".repeat(65 * 1024);
    {
        let res = client
            .post("/echo")
            .header("content-length", &(65 * 1024).to_string())
            .body(large_body.clone())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await.len(), 65 * 1024);
    }
}
