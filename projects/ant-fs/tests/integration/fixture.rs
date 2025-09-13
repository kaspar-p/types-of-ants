use ant_fs::make_routes;
use ant_library::axum_test_client::TestClient;

pub struct TestFixture {
    pub client: TestClient,
}

pub async fn test_router_no_auth() -> TestFixture {
    let api = make_routes().unwrap();

    TestFixture {
        client: TestClient::new(api).await,
    }
}

pub async fn test_router_auth() -> (TestFixture, String) {
    let fixture = test_router_no_auth().await;

    (fixture, "Basic dXNlcjp0ZXN0LXBhc3N3b3Jk".to_string()) // user:test-password
}
