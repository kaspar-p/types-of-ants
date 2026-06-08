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

#[tokio::test]
#[traced_test]
async fn route_delete_returns_4xx_if_unauthenticated() {
    let fixture = test_router_no_auth(function_name!()).await;

    {
        let res = fixture.client.delete("/file.txt").send().await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let res = fixture
            .client
            .delete("/file.txt")
            .header("Authorization", "Basic dXNlcjp0ZXN0LXBhc3N3b3JkMg==") // user:test-password2
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn route_delete_returns_200_if_file_there() {
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
            .delete("/file.txt")
            .header("Authorization", &header)
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

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn route_delete_returns_404_if_file_not_there() {
    let (fixture, header) = test_router_auth(function_name!()).await;

    {
        let res = fixture
            .client
            .delete("/file.txt")
            .header("Authorization", &header)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn route_returns_200_for_nested_path() {
    let (fixture, header) = test_router_auth(function_name!()).await;

    // Nested directories that don't exist yet are created on demand.
    {
        let res = fixture
            .client
            .put("/backups/2024/may/dump.sql")
            .header("Authorization", &header)
            .body("nested content".as_bytes())
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let res = fixture
            .client
            .get("/backups/2024/may/dump.sql")
            .header("Authorization", &header)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await, "nested content".as_bytes());
    }
}

#[tokio::test]
#[traced_test]
async fn route_returns_200_for_path_with_special_characters() {
    let (fixture, header) = test_router_auth(function_name!()).await;

    // The request path is hashed to a fixed hex filename on disk, so special
    // characters and traversal-looking segments are stored safely and can't
    // escape the namespace. They round-trip transparently.
    let path = "/release@v1.2.3+build,final..weird.tar.gz";

    {
        let res = fixture
            .client
            .put(path)
            .header("Authorization", &header)
            .body("special content".as_bytes())
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let res = fixture
            .client
            .get(path)
            .header("Authorization", &header)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await, "special content".as_bytes());
    }
}

#[tokio::test]
#[traced_test]
async fn route_namespaces_files_by_user() {
    // user1 from the fixture; user2 added to the test secret.
    let (fixture, user1) = test_router_auth(function_name!()).await;
    let user2 = "Basic dXNlcjI6dGVzdC1wYXNzd29yZDI="; // user2:test-password2

    // Both users write the same path with different content.
    {
        let res = fixture
            .client
            .put("/shared.txt")
            .header("Authorization", &user1)
            .body("content-one".as_bytes())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
    {
        let res = fixture
            .client
            .put("/shared.txt")
            .header("Authorization", user2)
            .body("content-two".as_bytes())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    // Each user reads back only their own content; no collision.
    {
        let res = fixture
            .client
            .get("/shared.txt")
            .header("Authorization", &user1)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await, "content-one".as_bytes());
    }
    {
        let res = fixture
            .client
            .get("/shared.txt")
            .header("Authorization", user2)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await, "content-two".as_bytes());
    }
}
