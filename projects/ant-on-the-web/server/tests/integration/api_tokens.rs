use ant_on_the_web::api_tokens::GrantTokenRequest;
use http::StatusCode;
use tracing_test::traced_test;

use crate::fixture::{test_router_admin_auth, test_router_auth, FixtureOptions};

#[tokio::test]
#[traced_test]
async fn api_tokens_token_post_returns_401_if_user_not_admin() {
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

    {
        let req = GrantTokenRequest {
            username: "user".to_string(),
        };
        let res = fixture
            .client
            .post("/api/api-tokens/token")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn api_tokens_token_post_returns_404_for_not_existing_user() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    {
        let req = GrantTokenRequest {
            username: "someone-else".to_string(),
        };
        let res = fixture
            .client
            .post("/api/api-tokens/token")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn api_tokens_token_post_returns_200_for_existing_user() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    {
        let req = GrantTokenRequest {
            username: "nobody".to_string(),
        };
        let res = fixture
            .client
            .post("/api/api-tokens/token")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}
