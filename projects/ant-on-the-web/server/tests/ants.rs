use ant_on_the_web::ants::{ReleasedAntsResponse, SuggestionRequest, TotalResponse};
use fixture::{authn_test_router, test_router};
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

    assert!(ants.ants.len() <= 1000); // page size
    assert!(ants.ants.len() <= total.total);

    let mut running_total = ants.ants.len();
    let mut has_next_page = true;
    let mut next_page = 1;
    while has_next_page {
        let ants_res = fixture
            .client
            .get(format!("/api/ants/released-ants?page={next_page}").as_str())
            .send()
            .await;
        assert_eq!(ants_res.status(), StatusCode::OK);
        let ants: ReleasedAntsResponse = ants_res.json().await;

        running_total += ants.ants.len();
        if ants.has_next_page {
            next_page += 1;
        } else {
            has_next_page = false;
        }
    }

    assert_eq!(running_total, total.total);
}

#[tokio::test]
#[traced_test]
async fn ants_suggest_returns_200_with_user_if_authenticated() {
    let (fixture, cookie) = authn_test_router().await;

    {
        let req = SuggestionRequest {
            suggestion_content: "some ant content".to_string(),
        };
        let res = fixture
            .client
            .post("/api/ants/suggest")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_suggest_returns_200_even_if_not_authenticated() {
    let fixture = test_router().await;

    let req = SuggestionRequest {
        suggestion_content: "some ant content".to_string(),
    };
    let res = fixture
        .client
        .post("/api/ants/suggest")
        .json(&req)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}
