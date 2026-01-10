use std::path::PathBuf;

use ant_host_agent::routes::service::InstallServiceRequest;
use hyper::StatusCode;
use reqwest::multipart::Form;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture::TestFixture;

#[test]
#[traced_test]
async fn service_installation_smoke() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let req = InstallServiceRequest {
            project: "proj1".to_string(),
            version: "v1".to_string(),
            is_docker: Some(false),
            secrets: Some(vec![
                "test-secret1".to_string(),
                "test-secret2.secret".to_string(),
            ]),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[test]
#[traced_test]
async fn service_installation_fails_invalid_inputs() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let req = InstallServiceRequest {
            project: "bad-proj1".to_string(),
            version: "v1".to_string(),
            is_docker: Some(false),
            secrets: Some(vec![
                "test-secret1".to_string(),
                "test-secret2.secret".to_string(),
            ]),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = InstallServiceRequest {
            project: "proj1".to_string(),
            version: "bad-v1".to_string(),
            is_docker: Some(false),
            secrets: Some(vec![
                "test-secret1".to_string(),
                "test-secret2.secret".to_string(),
            ]),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = InstallServiceRequest {
            project: "proj1".to_string(),
            version: "bad-v1".to_string(),
            is_docker: Some(false),
            secrets: Some(vec![
                "bad-test-secret1".to_string(),
                "test-secret2.secret".to_string(),
            ]),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[test]
#[traced_test]
async fn service_installation_docker_smoke() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let req = InstallServiceRequest {
            project: "docker-proj1".to_string(),
            version: "v1".to_string(),
            is_docker: Some(true),
            secrets: None,
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[test]
#[traced_test]
async fn service_registration_smoke() {
    let fixture = TestFixture::new(function_name!(), Some(true)).await;

    {
        let proj_file = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("integration")
            .join("archives")
            .join("deployment.docker-proj1.v1.tar.gz");

        let req = Form::new().file("file", proj_file).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Project", "docker-proj1")
            .header("X-Ant-Version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
