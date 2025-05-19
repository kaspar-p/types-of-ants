use ant_on_the_web::ants::{ReleasedAntsResponse, TotalResponse};
use fixture::test_router;
use http::StatusCode;
use tracing_test::traced_test;

mod fixture;

#[tokio::test]
#[traced_test]
async fn ants_total_matches_ants_released() {
    let fixture = test_router().await;

    let ants_res = fixture
        .client
        .get("/api/ants/released-ants?page=0")
        .send()
        .await;
    assert_eq!(ants_res.status(), StatusCode::OK);
    let ants: ReleasedAntsResponse = ants_res.json().await;

    let total_res = fixture.client.get("/api/ants/total").send().await;
    assert_eq!(total_res.status(), StatusCode::OK);
    let total: TotalResponse = total_res.json().await;

    assert_eq!(total.total, ants.ants.len());
}
