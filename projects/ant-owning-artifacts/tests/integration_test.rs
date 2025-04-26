use std::{thread::sleep, time::Duration};

use ant_owning_artifacts::{client::AntOwningArtifactsClient, start_server};
use tracing::info;

use ant_owning_artifacts::routes::make::MakeInput;
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn invalid_version_throws() {
    tokio::spawn(async {
        info!("Starting server...");
        start_server(Some(7007))
            .await
            .expect("Server failed to start!");
    });

    info!("Connecting client...");

    println!("testing!");

    let client = AntOwningArtifactsClient::new(None, Some(7007));

    while !client.healthy().await {
        info!("Waiting for server to be healthy...");
        sleep(Duration::from_millis(100));
    }

    let response = client
        .make(MakeInput {
            project_id: "ant-who-tweets".to_string(),
            project_version: "some-invalid-version".to_string(),
        })
        .await;

    info!("{:#?}", response);
    response.expect_err("Response was not an error!");
}

#[traced_test]
#[tokio::test]
async fn valid_version_builds_correctly() {
    tokio::spawn(async {
        info!("Starting server...");
        start_server(Some(7008))
            .await
            .expect("Server failed to start!");
    });

    info!("Connecting client...");

    println!("testing!");

    let client = AntOwningArtifactsClient::new(None, Some(7008));

    while !client.healthy().await {
        info!("Waiting for server to be healthy...");
        sleep(Duration::from_millis(100));
    }

    client
        .make(MakeInput {
            project_id: "ant-who-tweets".to_string(),
            project_version: "40863f7f300846dbb9a40fe365ec5dd10b22f94c".to_string(),
        })
        .await
        .expect("Works!");
}
