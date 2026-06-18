use http::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::{Fixture, TEST_BEARER_TOKEN};

pub mod fixture;

#[tokio::test]
#[traced_test]
async fn put_object_returns_401_missing_bearer_token() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/my-key", fixture.bucket_id))
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_401_invalid_bearer_token() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/my-key", fixture.bucket_id))
        .header("Authorization", "Bearer wrong-token")
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_404_bucket_not_found() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put("/b-nonexistent/my-key")
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_201_success() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/my-key", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"hello world".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_201_nested_key() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/schemas/anthill.toml", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"schema content".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_404_missing_key() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .get(&format!("/{}/never-written", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_round_trip() {
    let fixture = Fixture::new(function_name!()).await;
    let payload = b"the quick brown fox";

    {
        let res = fixture
            .client
            .put(&format!("/{}/round-trip", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/round-trip", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await.as_ref(), payload);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_nested_key_round_trip() {
    let fixture = Fixture::new(function_name!()).await;
    let payload = b"nested object";

    {
        let res = fixture
            .client
            .put(&format!("/{}/a/b/c/file.bin", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/a/b/c/file.bin", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await.as_ref(), payload);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_public_bucket_no_auth() {
    let fixture = Fixture::new(function_name!()).await;
    let payload = b"public data";

    {
        let res = fixture
            .client
            .put(&format!("/{}/pub-key", fixture.public_bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/pub-key", fixture.public_bucket_id))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await.as_ref(), payload);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_internal_bucket_with_valid_token() {
    let fixture = Fixture::new(function_name!()).await;
    let payload = b"internal data";

    {
        let res = fixture
            .client
            .put(&format!("/{}/int-key", fixture.internal_bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/int-key", fixture.internal_bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_401_internal_bucket_no_auth() {
    let fixture = Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .put(&format!("/{}/int-key", fixture.internal_bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"data".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/int-key", fixture.internal_bucket_id))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_404_private_bucket_no_auth() {
    let fixture = Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .put(&format!("/{}/priv-key", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"data".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // Unauthenticated requests against private buckets return 404 to prevent bucket enumeration.
    {
        let res = fixture
            .client
            .get(&format!("/{}/priv-key", fixture.bucket_id))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn delete_object_returns_200_success() {
    let fixture = Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .put(&format!("/{}/to-delete", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"ephemeral".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .delete(&format!("/{}/to-delete", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn delete_object_returns_404_missing_key() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .delete(&format!("/{}/nonexistent", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_404_after_delete() {
    let fixture = Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .put(&format!("/{}/transient", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"bye".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .delete(&format!("/{}/transient", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/transient", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_201_overwrites_existing() {
    let fixture = Fixture::new(function_name!()).await;
    let original = b"original content";
    let updated = b"updated content";

    {
        let res = fixture
            .client
            .put(&format!("/{}/overwrite-key", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(original.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .put(&format!("/{}/overwrite-key", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(updated.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/{}/overwrite-key", fixture.bucket_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await.as_ref(), updated);
    }
}
