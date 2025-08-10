use http::StatusCode;
use tracing_test::traced_test;

use crate::fixture::{get_telemetry_cookie, test_router_no_auth};

#[tokio::test]
#[traced_test]
async fn static_files_returns_200_and_gets_telemetry_cookies() {
    let fixture = test_router_no_auth().await;

    let res = fixture.client.get("/test-file.txt").send().await;

    assert_eq!(res.status(), StatusCode::OK);

    let cookie = get_telemetry_cookie(res.headers());

    assert!(cookie.contains("typesofants_telemetry="));
    assert!(cookie.contains("SameSite"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));

    let content = res.text().await;
    assert!(content.contains("Some test data!"));
}
