use fixture::test_router;
use http::StatusCode;

mod fixture;

#[tokio::test]
async fn ping_works() {
    let fixture = test_router().await;

    let res = fixture.client.get("/ping").send().await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await, "healthy ant");
}
