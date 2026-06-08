use crate::fixture::{test_router_no_auth, FixtureOptions};
use assertables::{assert_contains, assert_lt};
use http::StatusCode;

#[tokio::test]
async fn api_fallback_returns_404_with_sorted_route_list() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    let res = fixture
        .client
        .get("/api/totally-unknown-route")
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let body = res.text().await;

    // Spot-check a few routes that must always be present.
    assert_contains!(body, "GET /version");
    assert_contains!(body, "GET /ants/latest-ants");
    assert_contains!(body, "POST /ants/suggest");
    assert_contains!(body, "POST /users/login");
    assert_contains!(body, "POST /webhooks/stripe");

    // Assert that the list is sorted: "GET /ants/..." comes before "POST /ants/...".
    let get_ants_pos = body.find("GET /ants/").expect("GET /ants/ not found");
    let post_ants_pos = body.find("POST /ants/").expect("POST /ants/ not found");
    assert_lt!(get_ants_pos, post_ants_pos);
}
