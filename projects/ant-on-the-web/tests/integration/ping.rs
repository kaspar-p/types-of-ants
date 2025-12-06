use crate::fixture::{test_router_no_auth, FixtureOptions};
use http::StatusCode;

#[tokio::test]
async fn ping_works() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    let res = fixture.client.get("/ping").send().await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await, "healthy ant");
}
