use std::time::Duration;

use crate::fixture::ConsulFixture;
use ant_library::{sd::ServiceDiscovery, service::Service};
use http::StatusCode;
use serde_json::json;
use tokio::test;
use tracing::info;
use tracing_test::traced_test;

#[test]
#[traced_test]
async fn service_discovery_finds_no_service() {
    let consul = ConsulFixture::new().await;

    let sd = ServiceDiscovery::new(consul.port());

    let endpoint = sd.resolve(&Service::AntMatchmaker).await;

    assert!(endpoint.is_none());
}

#[test]
#[traced_test]
async fn service_discovery_finds_service_if_setup_before_itself() {
    let consul = ConsulFixture::new().await;

    let client = reqwest::Client::new();

    let res = client
        .put(format!(
            "http://localhost:{}/v1/agent/service/register",
            consul.port()
        ))
        .json(&json!({
          "Name": Service::AntMatchmaker.to_string(),
          "Address": "localhost",
          "Port": 20012
        }))
        .send()
        .await
        .expect("register failed to send")
        .error_for_status()
        .expect("register failed");

    assert_eq!(res.status(), StatusCode::OK);

    let sd = ServiceDiscovery::new(consul.port());

    let endpoint = sd.resolve(&Service::AntMatchmaker).await.unwrap();

    assert_eq!(endpoint.address, "localhost");
    assert_eq!(endpoint.port, 20012);

    info!("Finished");
}

#[test]
#[traced_test]
async fn service_discovery_finds_registered_service() {
    let consul = ConsulFixture::new().await;

    let sd = ServiceDiscovery::new(consul.port());

    let client = reqwest::Client::new();

    let endpoint = sd.resolve(&Service::AntMatchmaker).await;
    assert!(endpoint.is_none());

    let res = client
        .put(format!(
            "http://localhost:{}/v1/agent/service/register",
            consul.port()
        ))
        .json(&json!({
          "Name": Service::AntMatchmaker.to_string(),
          "Address": "localhost",
          "Port": 20012
        }))
        .send()
        .await
        .expect("register failed to send")
        .error_for_status()
        .expect("register failed");

    assert_eq!(res.status(), StatusCode::OK);

    // Give the refresher task some time to refresh the cache in the background!
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let endpoint = sd.resolve(&Service::AntMatchmaker).await.unwrap();

    assert_eq!(endpoint.address, "localhost");
    assert_eq!(endpoint.port, 20012);

    info!("Finished");
}
