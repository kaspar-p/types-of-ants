use http::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use ant_library::sd::writer::ServiceDiscoveryWriter;

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

#[tokio::test]
#[traced_test]
async fn put_object_returns_507_when_no_nodes_have_capacity() {
    let fixture = Fixture::new_with_capacity(function_name!(), 0).await;

    let res = fixture
        .client
        .put(&format!("/{}/any-key", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"hello".as_slice())
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::INSUFFICIENT_STORAGE);
}

#[tokio::test]
#[traced_test]
async fn upsert_placement_stores_all_replicas() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/placement-test", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let obj = fixture
        .db
        .get_object(&fixture.bucket_id, "placement-test")
        .await
        .unwrap()
        .unwrap();

    fixture
        .db
        .upsert_placement(&obj.object_id, "sn-test", &obj.object_id, "dummy-checksum", 1)
        .await
        .unwrap();

    let placements = fixture.db.get_placements(&obj.object_id).await.unwrap();
    assert_eq!(placements.len(), 2);
}

#[tokio::test]
#[traced_test]
async fn put_object_capacity_check_uses_consistent_size_units() {
    let fixture = Fixture::new_with_capacity(function_name!(), 120).await;

    let res1 = fixture
        .client
        .put(&format!("/{}/obj-a", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"12345678901234567890123456789012345678901234567890".as_slice())
        .send()
        .await;
    assert_eq!(res1.status(), StatusCode::CREATED);

    let res2 = fixture
        .client
        .put(&format!("/{}/obj-b", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"12345678901234567890123456789012345678901234567890".as_slice())
        .send()
        .await;
    assert_eq!(res2.status(), StatusCode::CREATED);

    let get_res = fixture
        .client
        .get(&format!("/{}/obj-b", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(get_res.status(), StatusCode::OK);
}

#[tokio::test]
#[traced_test]
async fn bytes_stored_excludes_soft_deleted_objects() {
    let fixture = Fixture::new_with_capacity(function_name!(), 55).await;

    let res1 = fixture
        .client
        .put(&format!("/{}/obj-to-delete", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"12345678901234567890123456789012345678901234567890".as_slice())
        .send()
        .await;
    assert_eq!(res1.status(), StatusCode::CREATED);

    let del = fixture
        .client
        .delete(&format!("/{}/obj-to-delete", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(del.status(), StatusCode::OK);

    let res2 = fixture
        .client
        .put(&format!("/{}/new-obj", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"0123456789".as_slice())
        .send()
        .await;
    assert_eq!(res2.status(), StatusCode::CREATED);

    let get_res = fixture
        .client
        .get(&format!("/{}/new-obj", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(get_res.status(), StatusCode::OK);
    assert_eq!(get_res.bytes().await.as_ref(), b"0123456789");
}

#[tokio::test]
#[traced_test]
async fn delete_object_returns_500_when_storage_node_unreachable() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/to-leak", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"sensitive data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);

        fixture.sd.stop_refreshing("ant-archive-storage").await;

    ServiceDiscoveryWriter::new(fixture.consul_port)
        .deregister_local_service("ant-archive-storage")
        .await
        .unwrap();

    let del = fixture
        .client
        .delete(&format!("/{}/to-leak", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;

    assert_eq!(del.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_with_replica_failover() {
    let fixture = Fixture::new(function_name!()).await;
    let payload = b"failover-payload";

    let res = fixture
        .client
        .put(&format!("/{}/failover-key", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(payload.as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let obj = fixture
        .db
        .get_object(&fixture.bucket_id, "failover-key")
        .await
        .unwrap()
        .unwrap();

    // Retrieve the real storage key and checksum written by the PUT.
    let placements = fixture.db.get_placements(&obj.object_id).await.unwrap();
    let real_key = &placements[0].storage_key;
    let real_checksum = &placements[0].object_checksum;

    // Add idx=1 with the real checksum, then corrupt idx=0's checksum.
    // get_object must skip idx=0 (checksum mismatch) and fall back to idx=1.
    fixture
        .db
        .upsert_placement(&obj.object_id, "sn-test", real_key, real_checksum, 1)
        .await
        .unwrap();
    fixture
        .db
        .upsert_placement(&obj.object_id, "sn-test", real_key, "BAD-CHECKSUM", 0)
        .await
        .unwrap();

    let res = fixture
        .client
        .get(&format!("/{}/failover-key", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.bytes().await.as_ref(), payload);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_400_when_required_node_unknown() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/any-key", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .header("X-Ant-Capability-Can-Select-Storage-Node", "nonexistent-node")
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[traced_test]
async fn put_object_places_on_requested_node() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture
        .client
        .put(&format!("/{}/pinned-key", fixture.bucket_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .header("X-Ant-Capability-Can-Select-Storage-Node", "sn-test")
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let obj = fixture
        .db
        .get_object(&fixture.bucket_id, "pinned-key")
        .await
        .unwrap()
        .unwrap();
    let placements = fixture.db.get_placements(&obj.object_id).await.unwrap();
    assert!(placements.iter().any(|p| p.storage_node_id == "sn-test"));
}
