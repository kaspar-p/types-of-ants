use std::time::Duration;

use ant_library::sd::{ServiceDiscovery, ServiceDiscoveryWriter};
use ant_library_test::consul_fixture::ConsulFixture;
use tokio::test;
use tracing_test::traced_test;

#[test]
#[traced_test]
async fn service_discovery_finds_no_service() {
    let consul = ConsulFixture::new().await;

    let sd = ServiceDiscovery::new(consul.port());

    let endpoint = sd.resolve("ant-matchmaker").await;

    assert!(endpoint.is_none());
}

#[test]
#[traced_test]
async fn service_discovery_finds_service_if_writer_before_reader() {
    let consul = ConsulFixture::new().await;

    let sd_writer = ServiceDiscoveryWriter::new(consul.port());

    sd_writer
        .register_service("ant-matchmaker", 20012)
        .await
        .unwrap();

    let sd = ServiceDiscovery::new(consul.port());

    let endpoint = sd.resolve("ant-matchmaker").await.unwrap();

    assert_eq!(
        endpoint.address,
        local_ip_address::local_ip().unwrap().to_string()
    );
    assert_eq!(endpoint.port, 20012);
}

#[test]
#[traced_test]
async fn service_discovery_finds_if_reader_before_writer() {
    let consul = ConsulFixture::new().await;

    let sd = ServiceDiscovery::new(consul.port());

    let endpoint = sd.resolve("ant-matchmaker").await;
    assert!(endpoint.is_none());

    let sd_writer = ServiceDiscoveryWriter::new(consul.port());

    sd_writer
        .register_service("ant-matchmaker", 20013)
        .await
        .unwrap();

    // Give the refresher task some time to refresh the cache in the background!
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let endpoint = sd.resolve("ant-matchmaker").await.unwrap();

    assert_eq!(
        endpoint.address,
        local_ip_address::local_ip().unwrap().to_string()
    );
    assert_eq!(endpoint.port, 20013);
}

#[test]
#[traced_test]
async fn service_discovery_finds_nothing_when_removed() {
    let consul = ConsulFixture::new().await;

    let sd = ServiceDiscovery::new(consul.port());

    let endpoint = sd.resolve("ant-matchmaker").await;
    assert!(endpoint.is_none());

    let sd_writer = ServiceDiscoveryWriter::new(consul.port());

    sd_writer
        .register_service("ant-matchmaker", 20013)
        .await
        .unwrap();

    // Give the refresher task some time to refresh the cache in the background!
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let endpoint = sd.resolve("ant-matchmaker").await.unwrap();

    assert_eq!(
        endpoint.address,
        local_ip_address::local_ip().unwrap().to_string()
    );
    assert_eq!(endpoint.port, 20013);

    sd_writer
        .deregister_service("ant-matchmaker")
        .await
        .unwrap();

    // Give the refresher task some time to refresh the cache in the background!
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let endpoint = sd.resolve("ant-matchmaker").await;
    assert!(endpoint.is_none());

    // Initialize a new writer so the previous cache is cleared, too.
    let sd2 = ServiceDiscovery::new(consul.port());
    let endpoint = sd2.resolve("ant-matchmaker").await;
    assert!(endpoint.is_none());
}
