use http::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::{test_router_auth, test_router_no_auth};

pub mod fixture;

#[tokio::test]
#[traced_test]
async fn route_returns_4xx_if_unauthenticated() {
    let fixture = test_router_no_auth(function_name!()).await;

    {
        let res = fixture.client.get("/file.txt").send().await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let res = fixture
            .client
            .get("/file.txt")
            .header("Authorization", "Basic dXNlcjp0ZXN0LXBhc3N3b3JkMg==") // user:test-password2
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn route_returns_404_if_file_not_there() {
    let (fixture, header) = test_router_auth(function_name!()).await;

    {
        let res = fixture
            .client
            .get("/never-there.txt")
            .header("Authorization", &header)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn route_returns_200_if_file_there() {
    let (fixture, header) = test_router_auth(function_name!()).await;

    {
        let res = fixture
            .client
            .put("/file.txt")
            .header("Authorization", &header)
            .body("some content here".as_bytes())
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let res = fixture
            .client
            .get("/file.txt")
            .header("Authorization", &header)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await, "some content here".as_bytes());
    }
}
