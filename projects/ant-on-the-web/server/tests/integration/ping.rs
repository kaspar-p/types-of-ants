use crate::fixture::test_router_no_auth;
use http::StatusCode;

#[tokio::test]
async fn ping_works() {
    let fixture = test_router_no_auth().await;

    let res = fixture.client.get("/ping").send().await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await, "healthy ant");
}
