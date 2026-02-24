use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use ant_zookeeper::routes::{
    pipeline::{
        AddHostToHostGroupRequest, CreateHostGroupRequest, CreateHostGroupResponse,
        PutPipelineRequest, PutPipelineStage,
    },
    service::{ProjectEnvironmentVariable, PutProjectEnvironmentRequest},
};
use ant_zookeeper_db::{Deployment, DeploymentJob};
use http::StatusCode;
use reqwest::multipart::Form;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture::Fixture;

async fn get_events(fixture: &Fixture, project: &str, revision_id: &str) -> Vec<Deployment> {
    fixture
        .state
        .db
        .list_deployment_events_in_pipeline_revision(project, &revision_id)
        .await
        .unwrap()
}

async fn get_unfinished_jobs(fixture: &Fixture) -> Vec<DeploymentJob> {
    fixture
        .state
        .db
        .list_unfinished_deployment_jobs()
        .await
        .unwrap()
}

async fn beta_stage_setup(fixture: &Fixture) {
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
}

async fn artifact_build_setup(fixture: &Fixture, version: Option<&str>) -> String {
    let version = version.unwrap_or("v1");

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
            .header("x-ant-version", version)
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
    assert_eq!(rev.project_id, "ant-host-agent");
    assert!(revisions.len() >= 1);
    let revision_id = rev.id.clone();

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

    {
        let jobs = get_unfinished_jobs().await;
        assert_eq!(jobs.len(), 0);

        let e = get_events().await;
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        if e.len() == 5 {
            assert_eq!(e, vec![]);
        }
    }

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(get_unfinished_jobs().await.len(), 2); // aarch64 already done above.
        let e = get_events().await;
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
    }

    // Iterate pipeline once
    {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        // Same status, since 2 remaining architecture jobs are forever pending.
        // The build stage's StageFinished event is on the frontier, but not READY because the arch
        // tasks are pointing to it still.
        assert_eq!(get_unfinished_jobs().await.len(), 2); // aarch64 already done above.
        let e = get_events().await;
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
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
            .header("x-ant-version", version)
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
        let e = get_events().await;
        assert_eq!(e.len(), 4);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e[3].event_name, "artifact-architecture-registered:x86_64");
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
            .header("x-ant-version", version)
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
        let e = get_events().await;
        assert_eq!(e.len(), 5);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e[3].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e[4].event_name, "artifact-architecture-registered:armv7");
    }

    revision_id
}

#[test]
#[traced_test]
async fn deployment_deployment_returns_200_happy_path() {
    let fixture = Fixture::new(function_name!()).await;

    beta_stage_setup(&fixture).await;
    let revision_id = artifact_build_setup(&fixture, None).await;
    let get_unfinished_jobs = || get_unfinished_jobs(&fixture);
    let get_events = || get_events(&fixture, "ant-host-agent", &revision_id);

    // Iterate pipeline 5 times, before requiring a deployment (fails locally, no systemd on MacOS)
    {
        for _ in 0..5 {
            let res = fixture.client.post("/deployment/iteration").send().await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        assert_eq!(get_unfinished_jobs().await.len(), 0);
        let e = get_events().await;
        assert_eq!(e.len(), 10);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e[3].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e[4].event_name, "artifact-architecture-registered:armv7");
        assert_eq!(e[5].event_name, "stage-finished");
        assert_eq!(e[6].event_name, "stage-started");
        assert_eq!(e[7].event_name, "host-group-started");
        assert_eq!(e[8].event_name, "host-started");
        assert_eq!(e[9].event_name, "host-artifact-replicated");
    }
}

#[test]
#[traced_test]
/**
 * Tests a scenario like:
 *  - Revision 1 starts
 *  - Revision 1 completes build, fails on deployment to host A
 *  - Revision 2 starts (bugfix)
 *  - Revision 2 completes build, succeeds deployment to host A
 *  - ... time passes
 *  - The pipeline might auto-retry Revision 1 and rollback accidentally.
 *
 * That is, the system should SURPASS Revision 1 and no longer consider it once some new revision
 * has completed successfully there.
 */
async fn deployment_deployment_returns_200_and_filters_revisions_if_newer_have_surpassed_it() {
    let fixture = Fixture::new(function_name!()).await;

    beta_stage_setup(&fixture).await;

    let v1_revision_id = artifact_build_setup(&fixture, Some("v1")).await;
    let get_unfinished_jobs = || get_unfinished_jobs(&fixture);
    let get_events_v1 = || get_events(&fixture, "ant-host-agent", &v1_revision_id);

    let (pipeline_id, build_stage_id) = {
        let e = get_events_v1().await;
        assert_eq!(get_unfinished_jobs().await.len(), 0);
        assert_eq!(e.len(), 5);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e[3].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e[4].event_name, "artifact-architecture-registered:armv7");

        (e[0].target_id.clone(), e[4].target_id.clone())
    };

    // == PAUSE REVISION 1 HERE, MAKE SURE THE NEXT STEP FAILS ==
    let job_id = {
        let job_id = fixture
            .state
            .db
            .create_deployment_job(
                &v1_revision_id,
                "ant-host-agent",
                &pipeline_id,
                "stage",
                &build_stage_id,
                "stage-finished",
            )
            .await
            .unwrap();
        fixture
            .state
            .db
            .complete_deployment_job(
                &job_id,
                &v1_revision_id,
                "stage",
                &build_stage_id,
                "stage-finished",
                false,
            )
            .await
            .unwrap();

        // Ensure that nothing progresses, since there's a failed deployment job for that
        {
            // Iterate many times, each time blocked by previously un-retryable failing job
            for _ in 0..5 {
                let res = fixture.client.post("/deployment/iteration").send().await;
                assert_eq!(res.status(), StatusCode::OK);
            }

            let e = get_events_v1().await;
            assert_eq!(get_unfinished_jobs().await.len(), 0);
            assert_eq!(e.len(), 5);
            assert_eq!(e[0].event_name, "pipeline-started");
            assert_eq!(e[1].event_name, "stage-started");
            assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
            assert_eq!(e[3].event_name, "artifact-architecture-registered:x86_64");
            assert_eq!(e[4].event_name, "artifact-architecture-registered:armv7");
        }

        job_id
    };

    let v2_revision_id = artifact_build_setup(&fixture, Some("v2")).await;
    let get_events_v2 = || get_events(&fixture, "ant-host-agent", &v2_revision_id);

    // Iterate pipeline twice to "surpass" the revision 1
    {
        for _ in 0..2 {
            let res = fixture.client.post("/deployment/iteration").send().await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        let e = get_events_v2().await;
        assert_eq!(get_unfinished_jobs().await.len(), 0);
        assert_eq!(e.len(), 7);
        assert_eq!(e[0].event_name, "pipeline-started");
        assert_eq!(e[1].event_name, "stage-started");
        assert_eq!(e[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e[3].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e[4].event_name, "artifact-architecture-registered:armv7");
        assert_eq!(e[5].event_name, "stage-finished");
        assert_eq!(e[6].event_name, "stage-started");
    }

    // UNPAUSE REVISION 1: by marking the failed deployment job as "retryable", so it will schedule a task
    // that succeeds immediately.
    {
        fixture
            .state
            .db
            .set_deployment_job_retryable(&job_id)
            .await
            .unwrap()
    }

    // Iterate pipeline
    {
        for _ in 0..2 {
            let res = fixture.client.post("/deployment/iteration").send().await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        assert_eq!(get_unfinished_jobs().await.len(), 0);

        // v1 hasn't moved
        let e1 = get_events_v1().await;
        assert_eq!(e1.len(), 5);
        assert_eq!(e1[0].event_name, "pipeline-started");
        assert_eq!(e1[1].event_name, "stage-started");
        assert_eq!(e1[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e1[3].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e1[4].event_name, "artifact-architecture-registered:armv7");

        // v2 has kept going
        let e2 = get_events_v2().await;
        assert_eq!(e2.len(), 9);
        assert_eq!(e2[0].event_name, "pipeline-started");
        assert_eq!(e2[1].event_name, "stage-started");
        assert_eq!(e2[2].event_name, "artifact-architecture-registered:aarch64");
        assert_eq!(e2[3].event_name, "artifact-architecture-registered:x86_64");
        assert_eq!(e2[4].event_name, "artifact-architecture-registered:armv7");
        assert_eq!(e2[5].event_name, "stage-finished");
        assert_eq!(e2[6].event_name, "stage-started");
        assert_eq!(e2[7].event_name, "host-group-started");
        assert_eq!(e2[8].event_name, "host-started");
    }
}
