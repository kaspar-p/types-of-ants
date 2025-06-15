use crate::fixture::test_router_no_auth;
use ant_on_the_web::hosts::{GetHostResponse, GetHostsResponse};
use http::StatusCode;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn getting_all_hosts_works() {
    let fixture = test_router_no_auth().await;

    let res = fixture.client.get("/api/hosts/hosts").send().await;

    assert_eq!(res.status(), StatusCode::OK);

    let hosts: GetHostsResponse = res.json().await;
    assert!(hosts.hosts.len() > 0);
}

#[tokio::test]
#[traced_test]
async fn getting_hostname_or_label() {
    let fixture = test_router_no_auth().await;

    let res1 = fixture
        .client
        .get("/api/hosts/host/antworker000")
        .send()
        .await;

    assert_eq!(res1.status(), StatusCode::OK);

    let hosts1: GetHostResponse = res1.json().await;
    assert_eq!(hosts1.host.host_label, "antworker000");
    assert_eq!(
        hosts1.host.host_hostname,
        "antworker000.hosts.typesofants.org"
    );
    assert_eq!(hosts1.host.host_type, "Raspberry Pi");
    assert_eq!(hosts1.host.host_os, "Rasbian");

    let res2 = fixture
        .client
        .get("/api/hosts/host/antworker000.hosts.typesofants.org")
        .send()
        .await;

    assert_eq!(res2.status(), StatusCode::OK);

    let hosts2: GetHostResponse = res2.json().await;
    assert_eq!(hosts1.host, hosts2.host);
}
