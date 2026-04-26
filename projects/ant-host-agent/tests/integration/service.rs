use ant_host_agent::routes::service::InstallServiceRequest;
use hyper::StatusCode;
use reqwest::multipart::Form;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture::TestFixture;

#[test]
#[traced_test]
async fn service_installation_fails_invalid_inputs() {
    let fixture = TestFixture::new(function_name!()).await;

    {
        let req = InstallServiceRequest {
            project: "bad-proj1".to_string(),
            version: "v1".to_string(),
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
async fn service_registration_plus_installation_smoke() {
    let fixture = TestFixture::new(function_name!()).await;

    // register
    {
        let file = fixture.make_tarfile_fixture("deployment.proj1.v1");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Project", "proj1")
            .header("X-Ant-Version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert!(std::fs::exists(
            fixture
                .test_root_dir
                .join("fs")
                .join("deployment.proj1.v1.tar.gz")
        )
        .unwrap())
    }

    // install
    {
        let req = InstallServiceRequest {
            project: "proj1".to_string(),
            version: "v1".to_string(),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let dir = fixture
            .test_root_dir
            .join("service")
            .join("proj1")
            .join("v1");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join("ant-host-agent.service")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());
    }
}

#[test]
#[traced_test]
async fn service_registration_plus_installation_docker_smoke() {
    let fixture = TestFixture::new(function_name!()).await;

    // register
    {
        let file = fixture.make_tarfile_fixture("deployment.docker-proj1.v1");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Project", "docker-proj1")
            .header("X-Ant-Version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert!(std::fs::exists(
            fixture
                .test_root_dir
                .join("fs")
                .join("deployment.docker-proj1.v1.tar.gz")
        )
        .unwrap())
    }

    // install
    {
        let req = InstallServiceRequest {
            project: "docker-proj1".to_string(),
            version: "v1".to_string(),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let dir = fixture
            .test_root_dir
            .join("service")
            .join("docker-proj1")
            .join("v1");
        assert!(std::fs::exists(dir.join("docker-compose.yml")).unwrap());
        assert!(std::fs::exists(dir.join("ant-gateway.service")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());
    }
}

#[test]
#[traced_test]
async fn service_registration_plus_installation_unversioned_smoke() {
    let fixture = TestFixture::new(function_name!()).await;

    // register
    {
        let file = fixture.make_tarfile_fixture("deployment.proj1.v1.global");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Project", "proj1")
            .header("X-Ant-Version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert!(std::fs::exists(
            fixture
                .test_root_dir
                .join("fs")
                .join("deployment.proj1.v1.tar.gz")
        )
        .unwrap())
    }

    // install globally / unversioned
    {
        let req = InstallServiceRequest {
            project: "proj1".to_string(),
            version: "v1".to_string(),
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let dir = fixture
            .test_root_dir
            .join("service")
            .join("proj1")
            .join("service");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join("ant-host-agent.service")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());
    }
}
