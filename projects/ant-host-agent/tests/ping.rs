use ant_host_agent::{
    clients::{Host, HostAgentClient},
    start_server,
};
use std::{thread::sleep, time::Duration};
use tracing::info;

use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn ping_healthy() {
    tokio::spawn(async {
        info!("Starting server...");
        start_server(Some(7008))
            .await
            .expect("Server failed to start!");
    });

    info!("Connecting...");

    let client = HostAgentClient::connect(Host {
        label: "test-host".to_string(),
        port: 7008,
        hostname: "localhost".to_string(),
    })
    .unwrap();

    while !client.healthy().await {
        info!("Waiting for server to be healthy...");
        sleep(Duration::from_millis(100));
    }

    client.ping().await.unwrap();

    ()
}
