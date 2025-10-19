use ant_host_agent::make_routes;
use ant_library::axum_test_client::TestClient;
use hyper::StatusCode;
use tracing::info;

use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn ping_healthy() {
    let fixture = TestClient::new(make_routes().await.unwrap()).await;

    info!("Connecting...");

    let response = fixture.get("/ping").send().await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await, "healthy ant");
}
