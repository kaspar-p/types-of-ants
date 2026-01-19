use std::path::PathBuf;

use ant_zookeeper::routes::{
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

    let get_unfinished_jobs = || async {
        fixture
            .state
            .db
            .list_unfinished_deployment_jobs()
            .await
            .unwrap()
    };

    let jobs = get_unfinished_jobs().await;
    assert_eq!(jobs.len(), 0);

    let events = get_events().await;

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].3, "stage-started");

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(get_unfinished_jobs().await.len(), 2); // aarch64 already done above.
        let events = get_events().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].3, "stage-started");
        assert_eq!(events[1].3, "artifact-architecture-registered:aarch64");
    }

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        // Same status, since 2 remaining architecture jobs are forever pending.
        // The build stage's StageFinished event is on the frontier, but not READY because the arch
        // tasks are pointing to it still.
        assert_eq!(get_unfinished_jobs().await.len(), 2); // aarch64 already done above.
        let events = get_events().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].3, "stage-started");
        assert_eq!(events[1].3, "artifact-architecture-registered:aarch64");
    }

    // Register the x86 artifact
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
            .header("x-ant-architecture", "x86")
            .header("x-ant-version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(get_unfinished_jobs().await.len(), 1); // only ArmV7 remaining
        let events = get_events().await;
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].3, "stage-started");
        assert_eq!(events[1].3, "artifact-architecture-registered:aarch64");
        assert_eq!(events[2].3, "artifact-architecture-registered:x86_64");
    }

    // Register the (FINAL) Raspian artifact
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
            .header("x-ant-architecture", "raspbian")
            .header("x-ant-version", "v1")
            .multipart(req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(get_unfinished_jobs().await.len(), 0);
        let events = get_events().await;
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].3, "stage-started");
        assert_eq!(events[1].3, "artifact-architecture-registered:aarch64");
        assert_eq!(events[2].3, "artifact-architecture-registered:x86_64");
        assert_eq!(events[3].3, "artifact-architecture-registered:armv7");
    }

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(get_unfinished_jobs().await.len(), 0);
        let events = get_events().await;
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].3, "stage-started");
        assert_eq!(events[1].3, "artifact-architecture-registered:aarch64");
        assert_eq!(events[2].3, "artifact-architecture-registered:x86_64");
        assert_eq!(events[3].3, "artifact-architecture-registered:armv7");
        assert_eq!(events[4].3, "stage-finished");
    }

    // Iterate pipeline 4 times, before requiring a deployment (fails locally, no systemd on MacOS)
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(get_unfinished_jobs().await.len(), 0);
        let events = get_events().await;
        assert_eq!(events.len(), 9);
        assert_eq!(events[0].3, "stage-started");
        assert_eq!(events[1].3, "artifact-architecture-registered:aarch64");
        assert_eq!(events[2].3, "artifact-architecture-registered:x86_64");
        assert_eq!(events[3].3, "artifact-architecture-registered:armv7");
        assert_eq!(events[4].3, "stage-finished");
        assert_eq!(events[5].3, "stage-started");
        assert_eq!(events[6].3, "host-group-started");
        assert_eq!(events[7].3, "host-started");
        assert_eq!(events[8].3, "host-artifact-replicated");
    }
}
