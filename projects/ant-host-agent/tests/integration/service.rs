use ant_host_agent::routes::service::InstallServiceRequest;
use hyper::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::TestFixture;

#[tokio::test]
#[traced_test]
async fn service_installation_smoke() {
    let fixture = TestFixture::new(function_name!()).await;

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

#[tokio::test]
#[traced_test]
async fn service_installation_fails_invalid_inputs() {
    let fixture = TestFixture::new(function_name!()).await;

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

#[tokio::test]
#[traced_test]
async fn service_installation_docker_smoke() {
    let fixture = TestFixture::new(function_name!()).await;

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
