use ant_host_agent::routes::service::InstallServiceRequest;
use assertables::assert_contains;
use hyper::StatusCode;
use reqwest::multipart::Form;
use serde_json::json;
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
            service_id: "bad-proj1".to_string(),
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
            service_id: "proj1".to_string(),
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
            .header("X-Ant-Service-Id", "proj1")
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
            service_id: "proj1".to_string(),
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
            .join("v1.1");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
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
            .header("X-Ant-Service-Id", "docker-proj1")
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
            service_id: "docker-proj1".to_string(),
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
            .join("v1.1");
        assert!(std::fs::exists(dir.join("docker-compose.yml")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());
    }
}

#[test]
#[traced_test]
async fn service_install_replaces_from_env_file_and_keeps_unknown_variables() {
    let fixture = TestFixture::new(function_name!()).await;

    // register
    {
        let file =
            fixture.make_tarfile_fixture("test-replaces-from-env-file-and-keeps-unknown-variables");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Service-Id", "ant-host-agent")
            .header("X-Ant-Version", "v8")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert!(std::fs::exists(
            fixture
                .test_root_dir
                .join("fs")
                .join("deployment.ant-host-agent.v8.tar.gz")
        )
        .unwrap())
    }

    // install
    {
        let req = InstallServiceRequest {
            service_id: "ant-host-agent".to_string(),
            version: "v8".to_string(),
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
            .join("ant-host-agent")
            .join("v8.1");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join("run.sh")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());

        let systemd_unit_content = std::fs::read_to_string(dir.join("run.sh")).unwrap();
        assert_contains!(
            systemd_unit_content,
            r#"./program --fake-data-directory "$PERSIST_DIR/fs""#
        );
    }
}

#[test]
#[traced_test]
async fn service_install_returns_200_with_same_destination_version() {
    let fixture = TestFixture::new(function_name!()).await;

    // register
    {
        let file = fixture.make_tarfile_fixture("deployment.proj1.v1");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Service-Id", "ant-host-agent")
            .header("X-Ant-Version", "v8")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert!(std::fs::exists(
            fixture
                .test_root_dir
                .join("fs")
                .join("deployment.ant-host-agent.v8.tar.gz")
        )
        .unwrap())
    }

    // install v8 (first attempt, so .1)
    {
        let req = InstallServiceRequest {
            service_id: "ant-host-agent".to_string(),
            version: "v8".to_string(),
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
            .join("ant-host-agent")
            .join("v8.1");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());

        assert!(std::fs::exists(dir.join("config.json")).unwrap());
        let config: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(dir.join("config.json")).unwrap()).unwrap();
        assert_eq!(config, json!({ "type": "normal" }));
    }

    // register different project as same version
    {
        let file = fixture.make_tarfile_fixture("deployment.proj1.v1-different");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Service-Id", "ant-host-agent")
            .header("X-Ant-Version", "v8")
            .multipart(req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert!(std::fs::exists(
            fixture
                .test_root_dir
                .join("fs")
                .join("deployment.ant-host-agent.v8.tar.gz")
        )
        .unwrap())
    }

    // install v8 (second attempt, so .2)
    {
        let req = InstallServiceRequest {
            service_id: "ant-host-agent".to_string(),
            version: "v8".to_string(),
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
            .join("ant-host-agent")
            .join("v8.2");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());

        assert!(std::fs::exists(dir.join("config.json")).unwrap());
        let config: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(dir.join("config.json")).unwrap()).unwrap();
        assert_eq!(config, json!({ "type": "different" }))
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
            .header("X-Ant-Service-Id", "proj1")
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
            service_id: "proj1".to_string(),
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
            .join("service.1");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());
    }
}
