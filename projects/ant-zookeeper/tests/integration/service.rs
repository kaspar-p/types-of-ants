use std::{
    fs::{create_dir_all, exists, File},
    io::Write,
    path::PathBuf,
};

use ant_zookeeper::routes::pipeline::{
    AddHostToHostGroupRequest, CreateHostGroupRequest, CreateHostGroupResponse, PutPipelineRequest,
    PutPipelineStage,
};
use http::StatusCode;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture;

fn digest(path: &PathBuf) -> String {
    sha256::try_digest(path).unwrap()
}

#[test]
#[traced_test]
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

#[test]
#[traced_test]
async fn service_artifact_returns_400_if_asking_for_unknown_secrets() {
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

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[test]
#[traced_test]
async fn service_artifact_returns_200_happy_path() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    // register secrets
    {
        let secret_root = fixture.state.root_dir.join("secrets-db");
        let paths = vec![
            secret_root.join("beta").join("tls_cert.secret"),
            secret_root.join("prod").join("tls_cert.secret"),
            secret_root.join("beta").join("tls_key.secret"),
            secret_root.join("prod").join("tls_key.secret"),
        ];
        for p in paths {
            create_dir_all(p.parent().unwrap()).unwrap();
            let mut file = File::create(p).unwrap();
            file.write_all("secret value".as_bytes()).unwrap();
        }
    }

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

#[test]
#[traced_test]
async fn service_artifact_includes_env_file() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    // register secrets
    {
        let secret_root = fixture.state.root_dir.join("secrets-db");
        let paths = vec![
            secret_root.join("beta").join("jwt.secret"),
            secret_root.join("prod").join("jwt.secret"),
        ];
        for p in paths {
            create_dir_all(p.parent().unwrap()).unwrap();
            let mut file = File::create(p).unwrap();
            file.write_all("secret value".as_bytes()).unwrap();
        }
    }

    // REPLICATE
    {
        // Create host group
        let host_group_id = {
            let req = CreateHostGroupRequest {
                name: "hg".to_string(),
                environment: "beta".to_string(),
            };

            let res = fixture
                .client
                .post("/pipeline/host-group/host-group")
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);

            let body: CreateHostGroupResponse = res.json().await;

            body.id
        };

        // Add 001 (aarch64) to hg
        {
            let req = AddHostToHostGroupRequest {
                host_group_id: host_group_id.clone(),
                host_id: "antworker001.hosts.typesofants.org".to_string(),
            };
            let res = fixture
                .client
                .post("/pipeline/host-group/host")
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);
        }

        // Create beta stage
        {
            let req = PutPipelineRequest {
                project: "ant-host-agent".to_string(),
                stages: vec![PutPipelineStage {
                    name: "beta-deployment".to_string(),
                    host_group_id: host_group_id.clone(),
                }],
            };

            let res = fixture
                .client
                .post("/pipeline/pipeline")
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);
        }

        // REGISTER
        let archive = {
            let archive = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
                .join("tests")
                .join("integration")
                .join("test-archives")
                .join("ant-host-agent-and-proj1-v1.tar.gz");
            let input_digest = digest(&archive);

            let req = reqwest::multipart::Form::new()
                .file("file", &archive)
                .await
                .unwrap();

            let res = fixture
                .client
                .post("/service/artifact")
                .header("X-Ant-Project", "ant-host-agent")
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
                .join("ant-host-agent.aarch64.v1.bld");

            assert_eq!(
                (output_path.clone(), exists(output_path.clone()).unwrap()),
                (output_path.clone(), true)
            );

            let output_digest = digest(&output_path);

            assert_eq!(input_digest, output_digest);

            // Iterate pipeline once
            {
                let res = fixture.client.post("/deployment/iteration").send().await;
                assert_eq!(res.status(), StatusCode::OK);
            }

            archive
        };

        // register other arches
        {
            let res = fixture
                .client
                .post("/service/artifact")
                .header("X-Ant-Project", "ant-host-agent")
                .header("X-Ant-Version", "v1")
                .header("X-Ant-Architecture", "x86")
                .multipart(
                    reqwest::multipart::Form::new()
                        .file("file", &archive)
                        .await
                        .unwrap(),
                )
                .send()
                .await;
            assert_eq!(res.status(), StatusCode::OK);

            // Iterate pipeline once
            {
                let res = fixture.client.post("/deployment/iteration").send().await;
                assert_eq!(res.status(), StatusCode::OK);
            }
        }

        {
            let res = fixture
                .client
                .post("/service/artifact")
                .header("X-Ant-Project", "ant-host-agent")
                .header("X-Ant-Version", "v1")
                .header("X-Ant-Architecture", "raspbian")
                .multipart(
                    reqwest::multipart::Form::new()
                        .file("file", &archive)
                        .await
                        .unwrap(),
                )
                .send()
                .await;
            assert_eq!(res.status(), StatusCode::OK);

            // Iterate pipeline once
            {
                let res = fixture.client.post("/deployment/iteration").send().await;
                assert_eq!(res.status(), StatusCode::OK);
            }
        }

        // Iterate pipeline once
        {
            let res = fixture.client.post("/deployment/iteration").send().await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        let revisions = fixture.state.db.list_revisions().await.unwrap();
        let rev = &revisions[0];
        assert_eq!(rev.1, "ant-host-agent");
        assert_eq!(revisions.len(), 1);
        let revision_id = rev.0.clone();

        let get_events = || async {
            fixture
                .state
                .db
                .list_deployment_events_in_pipeline_revision("ant-host-agent", &revision_id)
                .await
                .unwrap()
        };

        for _ in 0..4 {
            let res = fixture.client.post("/deployment/iteration").send().await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        let e = get_events().await;
        assert_eq!(e[0].event_name, "stage-started");
        assert_eq!(e[1].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e[3].event_name, "artifact-architecture-registered:armv7");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e[4].event_name, "stage-finished");
        assert_eq!(e[5].event_name, "stage-started");
        assert_eq!(e[6].event_name, "host-group-started");
        assert_eq!(e[7].event_name, "host-started");
        assert_eq!(e[8].event_name, "host-artifact-replicated");
        assert_eq!(e.len(), 9);

        // FINALLY, assert that the replication of the artifact on the host contains a .env file containing the "beta" fields
        let dir = fixture
            .ant_host_agent_state
            .install_root_dir
            .join("ant-host-agent")
            .join("v1");

        assert!(std::fs::exists(dir.join("ant-host-agent")).unwrap());
        assert!(std::fs::exists(dir.join("ant-host-agent.service")).unwrap());
        assert!(std::fs::exists(dir.join(".env")).unwrap());
        let env_file_content = std::fs::read_to_string(dir.join(".env")).unwrap();
        assert!(env_file_content.contains("TYPESOFANTS_ENV=beta"));
        assert!(env_file_content.contains("PERSIST_DIR=/home/ant/persist/ant-host-agent"));
        assert!(env_file_content.contains("ANT_HOST_AGENT_PORT=3232"));
    }
}
