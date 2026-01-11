use std::path::PathBuf;

use ant_zookeeper::routes::{
    deployment::CreateDeploymentRequest,
    pipeline::{
        AddHostToHostGroupRequest, CreateHostGroupRequest, CreateHostGroupResponse,
        PutPipelineRequest, PutPipelineStage,
    },
    service::{ProjectEnvironmentVariable, PutProjectEnvironmentRequest},
};
use http::StatusCode;
use reqwest::multipart::Form;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture::Fixture;

#[test]
#[traced_test]
async fn deployment_deployment_returns_200_happy_path() {
    let fixture = Fixture::new(function_name!()).await;

    // Create host group ant-host-agent/beta
    let host_group_id = {
        let req = CreateHostGroupRequest {
            name: "ant-host-agent/beta".to_string(),
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

    // Add 001 (arm) to ant-host-agent/beta
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

    // Create beta stage deploying to ant-host-agent/beta in pipeline
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

    // Register some dummy variables for the project in beta
    {
        let req = PutProjectEnvironmentRequest {
            project: "ant-host-agent".to_string(),
            environment: "beta".to_string(),
            variables: vec![ProjectEnvironmentVariable {
                key: "ANT_HOST_AGENT_PORT".to_string(),
                value: "3232".to_string(),
            }],
        };

        let res = fixture.client.post("/service/env").json(&req).send().await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // Register artifact for ant-host-agent on arm architecture
    {
        let archive = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("integration")
            .join("test-archives")
            .join("ant-host-agent-and-proj1-v1.tar.gz");
        let req = Form::new().file("file", archive).await.unwrap();

        let res = fixture
            .client
            .post("/service/artifact")
            .header("x-ant-project", "ant-host-agent")
            .header("x-ant-architecture", "arm")
            .header("x-ant-version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // Create deployment of new artifact to ant-host-agent/beta via the beta stage.
    {
        let req = CreateDeploymentRequest {
            project: "ant-host-agent".to_string(),
            host: "antworker001.hosts.typesofants.org".to_string(),
            version: "v1".to_string(),
        };

        let res = fixture
            .client
            .post("/deployment/deployment")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}
