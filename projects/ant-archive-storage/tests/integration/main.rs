use ant_archive_storage::make_metrics_routes;
use ant_library_test::axum_test_client::TestClient;
use http::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::{test_router_auth, test_router_no_auth};

pub mod fixture;

#[tokio::test]
#[traced_test]
async fn put_blob_returns_400_missing_auth_header() {
    let fixture = test_router_no_auth(function_name!()).await;

    let res = fixture.client.put("/some-key").send().await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[traced_test]
async fn put_blob_returns_401_wrong_credentials() {
    let fixture = test_router_no_auth(function_name!()).await;

    let res = fixture
        .client
        .put("/some-key")
        .header("Authorization", "Basic dXNlcjp3cm9uZy1wYXNzd29yZA==") // user:wrong-password
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_400_missing_auth_header() {
    let fixture = test_router_no_auth(function_name!()).await;

    let res = fixture.client.get("/some-key").send().await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_401_wrong_credentials() {
    let fixture = test_router_no_auth(function_name!()).await;

    let res = fixture
        .client
        .get("/some-key")
        .header("Authorization", "Basic dXNlcjp3cm9uZy1wYXNzd29yZA==")
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[traced_test]
async fn put_blob_returns_201_success() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let res = fixture
        .client
        .put("/my-key")
        .header("Authorization", &auth)
        .body(b"hello world".as_slice())
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_404_missing_key() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let res = fixture
        .client
        .get("/never-written")
        .header("Authorization", &auth)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_200_round_trip() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let payload = b"the quick brown fox";

    let put_res = fixture
        .client
        .put("/round-trip-key")
        .header("Authorization", &auth)
        .body(payload.as_slice())
        .send()
        .await;
    assert_eq!(put_res.status(), StatusCode::CREATED);

    let get_res = fixture
        .client
        .get("/round-trip-key")
        .header("Authorization", &auth)
        .send()
        .await;
    assert_eq!(get_res.status(), StatusCode::OK);
    assert_eq!(get_res.bytes().await.as_ref(), payload);
}

#[tokio::test]
#[traced_test]
async fn put_blob_uses_sharded_path_on_disk() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let key = "layout-test-key";
    fixture
        .client
        .put(&format!("/{key}"))
        .header("Authorization", &auth)
        .body(b"data".as_slice())
        .send()
        .await;

    let expected = ant_archive_storage::blob_path(&fixture.root, key);
    assert!(
        expected.exists(),
        "blob not at expected sharded path: {expected:?}"
    );

    // Verify two-level fan-out: blobs/<h[0..2]>/<h[2..4]>/<h>
    let components: Vec<_> = expected.components().collect();
    let n = components.len();
    let h2 = components[n - 3].as_os_str().to_str().unwrap();
    let h4 = components[n - 2].as_os_str().to_str().unwrap();
    let full = components[n - 1].as_os_str().to_str().unwrap();
    assert_eq!(h2.len(), 2, "first shard dir should be 2 hex chars");
    assert_eq!(h4.len(), 2, "second shard dir should be 2 hex chars");
    assert_eq!(&full[0..2], h2);
    assert_eq!(&full[2..4], h4);
}

#[tokio::test]
#[traced_test]
async fn put_blob_writes_encoding_v1_byte_on_disk() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let payload = b"some bytes";
    fixture
        .client
        .put("/encoding-test")
        .header("Authorization", &auth)
        .body(payload.as_slice())
        .send()
        .await;

    let path = ant_archive_storage::blob_path(&fixture.root, "encoding-test");
    let on_disk = std::fs::read(&path).expect("blob not found on disk");

    assert_eq!(on_disk[0], 1u8, "first byte must be encoding version 1");
    assert_eq!(&on_disk[1..], payload, "remainder must be raw payload");
}

#[tokio::test]
#[traced_test]
async fn head_blob_returns_200_logical_size() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let payload = b"exactly seventeen!!";
    fixture
        .client
        .put("/head-test")
        .header("Authorization", &auth)
        .body(payload.as_slice())
        .send()
        .await;

    let res = fixture
        .client
        .head("/head-test")
        .header("Authorization", &auth)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);

    let content_length: u64 = res
        .headers()
        .get("content-length")
        .expect("Content-Length header missing")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(content_length, payload.len() as u64);
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_206_range_read() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let payload = b"abcdefghij"; // 10 bytes
    fixture
        .client
        .put("/range-test")
        .header("Authorization", &auth)
        .body(payload.as_slice())
        .send()
        .await;

    // Request bytes 2-5 (inclusive) of the logical payload → "cdef"
    let res = fixture
        .client
        .get("/range-test")
        .header("Authorization", &auth)
        .header("Range", "bytes=2-5")
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(res.bytes().await.as_ref(), b"cdef");
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_206_open_ended_range() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let payload = b"abcdefghij";
    fixture
        .client
        .put("/range-open-test")
        .header("Authorization", &auth)
        .body(payload.as_slice())
        .send()
        .await;

    // bytes=5- means from offset 5 to end
    let res = fixture
        .client
        .get("/range-open-test")
        .header("Authorization", &auth)
        .header("Range", "bytes=5-")
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(res.bytes().await.as_ref(), b"fghij");
}

#[tokio::test]
#[traced_test]
async fn delete_blob_returns_200_success() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    fixture
        .client
        .put("/delete-me")
        .header("Authorization", &auth)
        .body(b"ephemeral".as_slice())
        .send()
        .await;

    let res = fixture
        .client
        .delete("/delete-me")
        .header("Authorization", &auth)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
#[traced_test]
async fn delete_blob_returns_404_missing_key() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    let res = fixture
        .client
        .delete("/nonexistent")
        .header("Authorization", &auth)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn get_blob_returns_404_after_delete() {
    let (fixture, auth) = test_router_auth(function_name!()).await;

    fixture
        .client
        .put("/transient")
        .header("Authorization", &auth)
        .body(b"bye".as_slice())
        .send()
        .await;

    fixture
        .client
        .delete("/transient")
        .header("Authorization", &auth)
        .send()
        .await;

    let res = fixture
        .client
        .get("/transient")
        .header("Authorization", &auth)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[traced_test]
async fn metrics_returns_200_with_prometheus_content() {
    let (fixture, _auth) = test_router_auth(function_name!()).await;
    let metrics = TestClient::new(make_metrics_routes(fixture.state.clone())).await;

    let res = metrics.get("/metrics").send().await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().await;
    assert!(
        body.contains("ant_archive_storage_bytes_stored"),
        "metrics body should contain bytes gauge: {body}"
    );
}

#[tokio::test]
#[traced_test]
async fn metrics_increments_counter_on_put_and_get() {
    let (fixture, auth) = test_router_auth(function_name!()).await;
    let metrics = TestClient::new(make_metrics_routes(fixture.state.clone())).await;

    fixture
        .client
        .put("/counter-test")
        .header("Authorization", &auth)
        .body(b"data".as_slice())
        .send()
        .await;

    fixture
        .client
        .get("/counter-test")
        .header("Authorization", &auth)
        .send()
        .await;

    let body = metrics.get("/metrics").send().await.text().await;
    assert!(
        body.contains("ant_archive_storage_http_requests_total"),
        "requests counter missing from metrics: {body}"
    );
    assert!(
        body.contains(r#"method="PUT""#),
        "PUT method label missing from metrics: {body}"
    );
    assert!(
        body.contains(r#"method="GET""#),
        "GET method label missing from metrics: {body}"
    );
}

#[tokio::test]
#[traced_test]
async fn metrics_records_request_duration_histogram() {
    let (fixture, auth) = test_router_auth(function_name!()).await;
    let metrics = TestClient::new(make_metrics_routes(fixture.state.clone())).await;

    fixture
        .client
        .put("/histogram-test")
        .header("Authorization", &auth)
        .body(b"data".as_slice())
        .send()
        .await;

    let body = metrics.get("/metrics").send().await.text().await;
    assert!(
        body.contains("ant_archive_storage_http_requests_duration_seconds"),
        "duration histogram missing from metrics: {body}"
    );
}

#[tokio::test]
#[traced_test]
async fn metrics_bytes_stored_increases_on_put() {
    let (fixture, auth) = test_router_auth(function_name!()).await;
    let metrics = TestClient::new(make_metrics_routes(fixture.state.clone())).await;

    fixture
        .client
        .put("/bytes-test")
        .header("Authorization", &auth)
        .body(b"hello".as_slice()) // 5 bytes
        .send()
        .await;

    let body = metrics.get("/metrics").send().await.text().await;
    assert!(
        body.contains("ant_archive_storage_bytes_stored 5"),
        "bytes gauge should be 5 after storing 'hello': {body}"
    );
}

#[tokio::test]
#[traced_test]
async fn metrics_bytes_stored_decreases_on_delete() {
    let (fixture, auth) = test_router_auth(function_name!()).await;
    let metrics = TestClient::new(make_metrics_routes(fixture.state.clone())).await;

    fixture
        .client
        .put("/bytes-delete-test")
        .header("Authorization", &auth)
        .body(b"hello".as_slice())
        .send()
        .await;

    fixture
        .client
        .delete("/bytes-delete-test")
        .header("Authorization", &auth)
        .send()
        .await;

    let body = metrics.get("/metrics").send().await.text().await;
    assert!(
        body.contains("ant_archive_storage_bytes_stored 0"),
        "bytes gauge should be 0 after delete: {body}"
    );
}

#[tokio::test]
#[traced_test]
async fn metrics_bytes_stored_adjusts_on_overwrite() {
    let (fixture, auth) = test_router_auth(function_name!()).await;
    let metrics = TestClient::new(make_metrics_routes(fixture.state.clone())).await;

    fixture
        .client
        .put("/overwrite-test")
        .header("Authorization", &auth)
        .body(b"hello".as_slice()) // 5 bytes
        .send()
        .await;

    fixture
        .client
        .put("/overwrite-test")
        .header("Authorization", &auth)
        .body(b"goodbye world".as_slice()) // 13 bytes
        .send()
        .await;

    let body = metrics.get("/metrics").send().await.text().await;
    assert!(
        body.contains("ant_archive_storage_bytes_stored 13"),
        "bytes gauge should be 13 after overwrite: {body}"
    );
}
