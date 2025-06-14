use crate::fixture::no_auth_test_router;
use http::StatusCode;

#[tokio::test]
async fn ping_works() {
    let fixture = no_auth_test_router().await;

    let res = fixture.client.get("/ping").send().await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await, "healthy ant");
}
