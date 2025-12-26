#[path = "../common/mod.rs"]
mod common;

use std::{fs::exists, path::PathBuf};

use http::StatusCode;
use tracing_test::traced_test;

use crate::deployments::common::fixture;

fn digest(path: &PathBuf) -> String {
    sha256::try_digest(path).unwrap()
}

//
#[traced_test]
#[tokio::test]
async fn register_service_version_returns_200() {
    let fixture = fixture::Fixture::new().await;

    let archive = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests")
        .join("integration")
        .join("test-archives")
        .join("deployment.docker-proj1.v1.tar.gz");
    let input_digest = digest(&archive);

    let req = reqwest::multipart::Form::new()
        .file("file", archive)
        .await
        .unwrap();

    let res = fixture
        .client
        .post("/services/service-version")
        .header("X-Ant-Project", "docker-proj1")
        .header("X-Ant-Version", "v1")
        .multipart(req)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);

    let output_path = fixture
        .state
        .root_dir
        .join("services-db")
        .join("docker-proj1.v1.bld");

    assert_eq!(
        (output_path.clone(), exists(output_path.clone()).unwrap()),
        (output_path.clone(), true)
    );

    let output_digest = digest(&output_path);

    assert_eq!(input_digest, output_digest);
}
