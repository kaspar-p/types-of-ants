use ant_host_agent::routes::service::InstallServiceRequest;
use assertables::assert_contains;
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
            project: None,
            service_id: Some("bad-proj1".to_string()),
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
            project: None,
            service_id: Some("proj1".to_string()),
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
            project: None,
            service_id: Some("proj1".to_string()),
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
            project: None,
            service_id: Some("docker-proj1".to_string()),
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
            project: None,
            service_id: Some("ant-host-agent".to_string()),
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
            .join("v8");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join("ant-host-agent.service")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());

        let systemd_unit_content =
            std::fs::read_to_string(dir.join("ant-host-agent.service")).unwrap();
        assert_contains!(
            systemd_unit_content,
            "--fake-data-directory /home/ant/persist/ant-host-agent/fs"
        );
        assert_contains!(systemd_unit_content, "--env beta");
        assert_contains!(systemd_unit_content, "--fake-serve-at-port=\":port1\"");
    }
}

#[test]
#[traced_test]
async fn service_install_backwards_compat_project_instead_of_service_id() {
    let fixture = TestFixture::new(function_name!()).await;

    // register
    {
        let file = fixture.make_tarfile_fixture("deployment.proj1.v1");

        let req = Form::new().file("file", file.path()).await.unwrap();

        let response = fixture
            .client
            .post("/service/service-registration")
            .header("X-Ant-Project", "ant-host-agent")
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
            project: Some("ant-host-agent".to_string()),
            service_id: None,
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
            .join("v8");
        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join("ant-host-agent.service")).unwrap());
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
            project: None,
            service_id: Some("proj1".to_string()),
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
