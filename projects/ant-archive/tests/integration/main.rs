use std::collections::HashMap;

use ant_archive::routes::{metadata::list_objects::ListObjectsResponse, vacuum::VacuumReport};
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
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/my-key", ids.private_id))
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_401_invalid_bearer_token() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/my-key", ids.private_id))
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
        .put("/o/b-nonexistent/my-key")
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
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/my-key", ids.private_id))
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
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/schemas/anthill.toml", ids.private_id))
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
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .get(&format!("/o/{}/never-written", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_round_trip() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;
    let payload = b"the quick brown fox";

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/round-trip", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/round-trip", ids.private_id))
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
    let ids = fixture.bucket_ids().await;
    let payload = b"nested object";

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/a/b/c/file.bin", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/a/b/c/file.bin", ids.private_id))
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
    let ids = fixture.bucket_ids().await;
    let payload = b"public data";

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/pub-key", ids.public_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/pub-key", ids.public_id))
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
    let ids = fixture.bucket_ids().await;
    let payload = b"internal data";

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/int-key", ids.internal_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(payload.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/int-key", ids.internal_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_404_internal_bucket_no_auth() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/int-key", ids.internal_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"data".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/int-key", ids.internal_id))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_404_private_bucket_no_auth() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/priv-key", ids.private_id))
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
            .get(&format!("/o/{}/priv-key", ids.private_id))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn delete_object_returns_200_success() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/to-delete", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"ephemeral".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .delete(&format!("/o/{}/to-delete", ids.private_id))
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
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .delete(&format!("/o/{}/nonexistent", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_404_after_delete() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/transient", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"bye".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .delete(&format!("/o/{}/transient", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/transient", ids.private_id))
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
    let ids = fixture.bucket_ids().await;
    let original = b"original content";
    let updated = b"updated content";

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/overwrite-key", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(original.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/overwrite-key", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(updated.as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/overwrite-key", ids.private_id))
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
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/any-key", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"hello".as_slice())
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::INSUFFICIENT_STORAGE);
}

#[tokio::test]
#[traced_test]
async fn put_object_capacity_check_uses_consistent_size_units() {
    let fixture = Fixture::new_with_capacity(function_name!(), 120).await;
    let ids = fixture.bucket_ids().await;

    let res1 = fixture
        .client
        .put(&format!("/o/{}/obj-a", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"12345678901234567890123456789012345678901234567890".as_slice())
        .send()
        .await;
    assert_eq!(res1.status(), StatusCode::CREATED);

    let res2 = fixture
        .client
        .put(&format!("/o/{}/obj-b", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"12345678901234567890123456789012345678901234567890".as_slice())
        .send()
        .await;
    assert_eq!(res2.status(), StatusCode::CREATED);

    let get_res = fixture
        .client
        .get(&format!("/o/{}/obj-b", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(get_res.status(), StatusCode::OK);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_507_deleted_object_not_yet_vacuumed() {
    let fixture = Fixture::new_with_capacity(function_name!(), 55).await;
    let ids = fixture.bucket_ids().await;

    put_object(
        &fixture,
        &ids.private_id,
        "obj-to-delete",
        "x".repeat(40).as_bytes(),
    )
    .await;
    delete_object(&fixture, &ids.private_id, "obj-to-delete").await;

    // Deleting only marks the object deletable, so its shards still fill the nodes.
    {
        let res = fixture
            .client
            .put(&format!("/o/{}/new-obj", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body("y".repeat(40))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::INSUFFICIENT_STORAGE);
    }

    vacuum(&fixture).await;

    put_object(
        &fixture,
        &ids.private_id,
        "new-obj",
        "y".repeat(40).as_bytes(),
    )
    .await;

    let get_res = fixture
        .client
        .get(&format!("/o/{}/new-obj", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(get_res.status(), StatusCode::OK);
    assert_eq!(get_res.text().await, "y".repeat(40));
}

#[tokio::test]
#[traced_test]
async fn delete_object_returns_200_even_when_storage_node_unreachable() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/to-leak", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(b"sensitive data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    // Remote the service from discovery
    {
        let endpoints = fixture.sd.resolve_all("ant-archive-storage").await;
        fixture.sd.stop_refreshing("ant-archive-storage").await;

        for e in endpoints {
            ServiceDiscoveryWriter::new(fixture.consul_port)
                .deregister_remote_service(&e.node, "ant-archive-storage")
                .await
                .unwrap();
        }
    }

    let del = fixture
        .client
        .delete(&format!("/o/{}/to-leak", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;

    assert_eq!(del.status(), StatusCode::OK);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_with_replica_failover() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;
    let payload = b"failover";

    let res = fixture
        .client
        .put(&format!("/o/{}/failover-key", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(payload.as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let obj = fixture
        .db
        .get_current_object(&ids.private_id, "failover-key")
        .await
        .unwrap()
        .unwrap();
    let chunks = fixture
        .db
        .list_chunks_for_object(&obj.object_id)
        .await
        .unwrap();
    let chunk = chunks.first().unwrap();

    // Retrieve the real storage key and checksum written by the PUT.
    let placements = fixture
        .db
        .list_chunk_shard_placements(&chunk.chunk_id)
        .await
        .unwrap();

    {
        // Corrupt a placement's checksum, it should fallback to other nodes
        fixture
            .db
            .upsert_shard_placement(
                &chunk.chunk_id,
                placements[0].shard_idx,
                &placements[0].storage_node_id,
                &placements[0].storage_key,
                payload.len() as i64,
                "BAD-CHECKSUM".as_bytes(),
            )
            .await
            .unwrap();
    }

    let res = fixture
        .client
        .get(&format!("/o/{}/failover-key", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.bytes().await.as_ref(), payload);
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_200_and_places_on_requested_node() {
    // It still chooses forced to choose that one if the header is supplied
    let mut map = HashMap::new();
    map.insert("sn-test1".to_string(), 0);
    map.insert("sn-test2".to_string(), 100);
    map.insert("sn-test3".to_string(), 100);
    let fixture = Fixture::new_with_capacities(function_name!(), map).await;
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/pinned-key", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .header("X-Ant-Capability-Can-Select-Storage-Node", "sn-test1")
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED, "{}", res.text().await);

    let obj = fixture
        .db
        .get_current_object(&ids.private_id, "pinned-key")
        .await
        .unwrap()
        .unwrap();

    let chunks = fixture
        .db
        .list_chunks_for_object(&obj.object_id)
        .await
        .unwrap();
    let chunk = chunks.first().unwrap();

    let placements = fixture
        .db
        .list_chunk_shard_placements(&chunk.chunk_id)
        .await
        .unwrap();

    assert!(placements.iter().any(|p| p.storage_node_id == "sn-test1"));
}

#[tokio::test]
#[traced_test]
async fn put_object_returns_400_if_no_such_requested_node() {
    // It still chooses forced to choose that one if the header is supplied
    let fixture = Fixture::new_with_capacity(function_name!(), 0).await;
    let ids = fixture.bucket_ids().await;

    let res = fixture
        .client
        .put(&format!("/o/{}/pinned-key", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .header("X-Ant-Capability-Can-Select-Storage-Node", "blah-blah")
        .body(b"data".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[traced_test]
async fn list_buckets_returns_401_missing_bearer_token() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture.client.get("/buckets").send().await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn list_buckets_returns_200_with_client_buckets() {
    let fixture = Fixture::new(function_name!()).await;

    let ids = fixture.bucket_ids().await;
    assert!(!ids.private_id.is_empty());
    assert!(!ids.internal_id.is_empty());
    assert!(!ids.public_id.is_empty());
    assert_ne!(ids.private_id, ids.internal_id);
    assert_ne!(ids.private_id, ids.public_id);
    assert_ne!(ids.internal_id, ids.public_id);
}

#[tokio::test]
#[traced_test]
async fn list_objects_returns_404_missing_prefix() {
    let fixture = Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .get(&format!(
                "/m/objects/{}",
                fixture.bucket_ids().await.public_id
            ))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn list_objects_returns_404_with_no_auth() {
    let fixture = Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .get(&format!(
                "/m/objects/{}/prefix1",
                fixture.bucket_ids().await.internal_id
            ))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    {
        let res = fixture
            .client
            .get(&format!(
                "/m/objects/{}/prefix2",
                fixture.bucket_ids().await.private_id
            ))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn list_objects_returns_200_with_objects() {
    let fixture = Fixture::new(function_name!()).await;

    let ids = fixture.bucket_ids().await;

    {
        let res = fixture
            .client
            .put(&format!("/o/{}/prefix1", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"sensitive data 1".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .put(&format!(
                "/o/{}/prefix2-and/then.some/stuff-afterwards",
                ids.private_id
            ))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"sensitive data 2".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }
    {
        let res = fixture
            .client
            .put(&format!(
                "/o/{}/prebad-and/then.some/stuff-afterwards",
                ids.private_id
            ))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"sensitive data 3".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    {
        let res = fixture
            .client
            .get(&format!("/m/objects/{}/pref", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let mut body: ListObjectsResponse = res.json().await;

        assert_eq!(body.objects.len(), 2);
        body.objects.sort_by(|a, b| a.key.cmp(&b.key));

        assert_eq!(body.objects[0].key, "prefix1");
        assert_eq!(
            body.objects[1].key,
            "prefix2-and/then.some/stuff-afterwards"
        );
    }
}

#[tokio::test]
#[traced_test]
async fn list_objects_returns_200_prefix_with_like_wildcards() {
    let fixture = Fixture::new(function_name!()).await;

    let ids = fixture.bucket_ids().await;

    for key in [
        "under_score",
        "underXscore",
        "100%-done",
        "100-percent",
        "back\\slash",
        "back%slash",
    ] {
        let res = fixture
            .client
            .put(&format!(
                "/o/{}/{}",
                ids.private_id,
                key.replace('%', "%25").replace('\\', "%5C")
            ))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .body(b"data".as_slice())
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    for (prefix, expected) in [
        ("under_", "under_score"),
        ("100%25", "100%-done"),
        ("back%5C", "back\\slash"),
    ] {
        let res = fixture
            .client
            .get(&format!("/m/objects/{}/{}", ids.private_id, prefix))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: ListObjectsResponse = res.json().await;

        assert_eq!(
            body.objects
                .iter()
                .map(|o| o.key.as_str())
                .collect::<Vec<_>>(),
            vec![expected],
            "prefix {prefix}"
        );
    }
}

async fn put_object(fixture: &Fixture, bucket_id: &str, key: &str, body: &[u8]) {
    let res = fixture
        .client
        .put(&format!("/o/{bucket_id}/{key}"))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .body(body.to_vec())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

async fn delete_object(fixture: &Fixture, bucket_id: &str, key: &str) {
    let res = fixture
        .client
        .delete(&format!("/o/{bucket_id}/{key}"))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}

async fn vacuum(fixture: &Fixture) -> VacuumReport {
    let res = fixture
        .client
        .post("/vacuum")
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await
}

#[tokio::test]
#[traced_test]
async fn vacuum_returns_401_missing_bearer_token() {
    let fixture = Fixture::new(function_name!()).await;

    let res = fixture.client.post("/vacuum").send().await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn vacuum_returns_200_purges_deleted_objects() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    // Each is one 10-byte chunk, replicated to 3 nodes.
    put_object(&fixture, &ids.private_id, "keep", b"abc").await;
    put_object(&fixture, &ids.private_id, "gone", b"xyz").await;
    assert_eq!(fixture.stored_blobs().len(), 6);

    delete_object(&fixture, &ids.private_id, "gone").await;

    let report = vacuum(&fixture).await;
    assert!(!report.skipped);
    assert_eq!(report.placements_purged, 3);
    assert_eq!(report.failures, Vec::<String>::new());
    assert_eq!(fixture.stored_blobs().len(), 3);

    let res = fixture
        .client
        .get(&format!("/o/{}/keep", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.bytes().await.as_ref(), b"abc");
}

#[tokio::test]
#[traced_test]
async fn vacuum_returns_200_purges_overwritten_versions() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    put_object(&fixture, &ids.private_id, "versioned", b"v1").await;
    put_object(&fixture, &ids.private_id, "versioned", b"v2").await;
    assert_eq!(fixture.stored_blobs().len(), 6);

    let report = vacuum(&fixture).await;
    assert_eq!(report.placements_purged, 3);
    assert_eq!(fixture.stored_blobs().len(), 3);

    let res = fixture
        .client
        .get(&format!("/o/{}/versioned", ids.private_id))
        .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.bytes().await.as_ref(), b"v2");
}

#[tokio::test]
#[traced_test]
async fn vacuum_returns_200_defers_recently_deleted_objects() {
    let fixture =
        Fixture::new_with_vacuum_grace(function_name!(), chrono::Duration::hours(1)).await;
    let ids = fixture.bucket_ids().await;

    put_object(&fixture, &ids.private_id, "just-deleted", b"abc").await;
    delete_object(&fixture, &ids.private_id, "just-deleted").await;

    let report = vacuum(&fixture).await;
    assert_eq!(report.placements_purged, 0);
    assert_eq!(report.deferred, 3);
    assert_eq!(fixture.stored_blobs().len(), 3);

    // Deferred work means the next vacuum can't skip, even with no new activity.
    let report = vacuum(&fixture).await;
    assert!(!report.skipped);
    assert_eq!(report.deferred, 3);
}

#[tokio::test]
#[traced_test]
async fn vacuum_returns_200_deletes_orphans_older_than_grace() {
    let fixture =
        Fixture::new_with_vacuum_grace(function_name!(), chrono::Duration::hours(1)).await;
    let ids = fixture.bucket_ids().await;

    put_object(&fixture, &ids.private_id, "placed", b"abc").await;
    let old = fixture.plant_orphan(
        "obj-failed-upload-00000000-0",
        std::time::Duration::from_secs(2 * 60 * 60),
    );
    let fresh = fixture.plant_orphan("obj-uploading-00000000-0", std::time::Duration::ZERO);

    let report = vacuum(&fixture).await;
    assert_eq!(report.orphans_deleted, 1);
    assert_eq!(report.deferred, 1);
    assert!(!old.exists(), "old orphan should be deleted");
    assert!(fresh.exists(), "fresh orphan may be an upload in progress");

    // Placed blobs are never orphans.
    assert_eq!(fixture.stored_blobs().len(), 3 + 1);
}

#[tokio::test]
#[traced_test]
async fn vacuum_returns_200_skipped_without_new_activity() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    put_object(&fixture, &ids.private_id, "first", b"abc").await;
    delete_object(&fixture, &ids.private_id, "first").await;

    let report = vacuum(&fixture).await;
    assert!(!report.skipped);
    assert_eq!(report.placements_purged, 3);

    let report = vacuum(&fixture).await;
    assert!(report.skipped);

    put_object(&fixture, &ids.private_id, "second", b"abc").await;

    let report = vacuum(&fixture).await;
    assert!(!report.skipped);
    assert_eq!(report.placements_purged, 0);

    let report = vacuum(&fixture).await;
    assert!(report.skipped);
}

#[tokio::test]
#[traced_test]
async fn get_object_returns_200_reuploaded_after_delete() {
    let fixture = Fixture::new(function_name!()).await;
    let ids = fixture.bucket_ids().await;

    put_object(&fixture, &ids.private_id, "phoenix", b"first").await;
    delete_object(&fixture, &ids.private_id, "phoenix").await;
    put_object(&fixture, &ids.private_id, "phoenix", b"second").await;

    {
        let res = fixture
            .client
            .get(&format!("/o/{}/phoenix", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.bytes().await.as_ref(), b"second");
    }

    {
        let res = fixture
            .client
            .get(&format!("/m/objects/{}/phoenix", ids.private_id))
            .header("Authorization", &format!("Bearer {TEST_BEARER_TOKEN}"))
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);

        let body: ListObjectsResponse = res.json().await;
        assert_eq!(
            body.objects
                .iter()
                .map(|o| o.key.as_str())
                .collect::<Vec<_>>(),
            vec!["phoenix"]
        );
    }
}
