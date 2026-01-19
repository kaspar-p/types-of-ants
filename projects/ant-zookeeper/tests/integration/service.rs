use std::{fs::exists, path::PathBuf};

use http::StatusCode;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture;

fn digest(path: &PathBuf) -> String {
    sha256::try_digest(path).unwrap()
}

#[traced_test]
#[test]
async fn service_artifact_returns_400_if_no_headers() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    {
        let req = reqwest::multipart::Form::new();
        let res = fixture
            .client
            .post("/service/artifact")
            // .header("X-Ant-Project", "docker-proj1")
            .header("X-Ant-Version", "v1")
            .header("X-Ant-Architecture", "aarch64")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = reqwest::multipart::Form::new();
        let res = fixture
            .client
            .post("/service/artifact")
            .header("X-Ant-Project", "docker-proj1")
            // .header("X-Ant-Version", "v1")
            .header("X-Ant-Architecture", "aarch64")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = reqwest::multipart::Form::new();
        let res = fixture
            .client
            .post("/service/artifact")
            .header("X-Ant-Project", "docker-proj1")
            .header("X-Ant-Version", "v1")
            // .header("X-Ant-Architecture", "aarch64")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // bad arch header
    {
        let req = reqwest::multipart::Form::new();
        let res = fixture
            .client
            .post("/service/artifact")
            .header("X-Ant-Project", "docker-proj1")
            .header("X-Ant-Version", "v1")
            .header("X-Ant-Architecture", "something-else")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[traced_test]
#[test]
async fn service_artifact_returns_200_happy_path() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    let archive = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests")
        .join("integration")
        .join("test-archives")
        .join("ant-gateway-v1.tar.gz");
    let input_digest = digest(&archive);

    let req = reqwest::multipart::Form::new()
        .file("file", archive)
        .await
        .unwrap();

    let res = fixture
        .client
        .post("/service/artifact")
        .header("X-Ant-Project", "ant-gateway")
        .header("X-Ant-Version", "v1")
        .header("X-Ant-Architecture", "aarch64")
        .multipart(req)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);

    let output_path = fixture
        .state
        .root_dir
        .join("artifacts-db")
        .join("ant-gateway.aarch64.v1.bld");

    assert_eq!(
        (output_path.clone(), exists(output_path.clone()).unwrap()),
        (output_path.clone(), true)
    );

    let output_digest = digest(&output_path);

    assert_eq!(input_digest, output_digest);
}
