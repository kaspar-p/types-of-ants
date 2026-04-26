use hyper::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::TestFixture;

#[traced_test]
#[tokio::test]
async fn ping_healthy() {
    let fixture = TestFixture::new(function_name!()).await;

    let response = fixture.client.get("/ping").send().await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await, "healthy ant");
}
