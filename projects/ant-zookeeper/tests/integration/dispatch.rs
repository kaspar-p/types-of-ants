use std::fs::{self, File};
use std::io::Write;

use ant_zookeeper::{
    pipeline::dispatch::dispatch,
    pipeline_engine::engine::{Dispatch, DispatchDirection, Node},
    routes::service::{UpsertRevisionRequest, UpsertRevisionResponse},
};
use http::StatusCode;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture::Fixture;

async fn upload_artifact(fixture: &Fixture, rev_id: &str, arch: &str, version: &str) {
    let archive = fixture.make_tarfile_fixture("ant-host-agent-and-proj1-v1");
    let req = reqwest::multipart::Form::new()
        .file("file", archive.path())
        .await
        .unwrap();
    let res = fixture
        .client
        .post("/service/artifact")
        .header("X-Ant-Revision", rev_id)
        .header("X-Ant-Project", "ant-host-agent")
        .header("X-Ant-Architecture", arch)
        .header("X-Ant-Version", version)
        .multipart(req)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}

async fn upsert_revision(fixture: &Fixture) -> String {
    let req = UpsertRevisionRequest {
        project: "ant-host-agent".to_string(),
    };
    let res = fixture
        .client
        .post("/service/revision")
        .json(&req)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    res.json::<UpsertRevisionResponse>().await.revision
}

fn artifact_replication_dispatch(revision_id: &str, host_id: &str, version: &str) -> Dispatch {
    let event = serde_json::json!({
        "type": "artifact_replication",
        "host_id": host_id,
        "service_id": "ant-host-agent",
        "environment": "beta",
    });
    let node = Node {
        node_id: format!("test-node-{version}"),
        revision_id: revision_id.to_string(),
        event: event.to_string(),
        state: "executable".to_string(),
        resource_key: None,
    };
    Dispatch {
        direction: DispatchDirection::Deploy,
        revision_id: revision_id.to_string(),
        node,
    }
}

#[test]
#[traced_test]
async fn dispatch_artifact_replication_registers_and_installs_service() {
    let fixture = Fixture::new(function_name!()).await;

    // replicate_artifact_step needs a tmp dir for tarball repacking
    tokio::fs::create_dir_all(fixture.state.root_dir.join("tmp"))
        .await
        .unwrap();

    // ant-host-agent-and-proj1-v1/anthill.json requires a "jwt" secret in all environments
    // (register_artifact validates secrets for both "beta" and "prod")
    for env in ["beta", "prod"] {
        let secret_path = fixture
            .state
            .root_dir
            .join("secrets-db")
            .join(env)
            .join("jwt.secret");
        fs::create_dir_all(secret_path.parent().unwrap()).unwrap();
        File::create(&secret_path)
            .unwrap()
            .write_all(b"secret")
            .unwrap();
    }

    let revision_id = upsert_revision(&fixture).await;
    // antworker001 has arch aarch64 in the seed data
    upload_artifact(&fixture, &revision_id, "aarch64", "v1").await;

    let d = artifact_replication_dispatch(&revision_id, "antworker001", "v1");

    dispatch(fixture.state.clone(), d).await.unwrap();

    assert!(
        fixture
            .ant_host_agent_state
            .archive_root_dir
            .join("deployment.ant-host-agent.v1.tar.gz")
            .exists(),
        "register_service should store the tarball"
    );
    assert!(
        fixture
            .ant_host_agent_state
            .install_root_dir
            .join("ant-host-agent")
            .join("v1.1")
            .exists(),
        "install_service should unpack to versioned dir"
    );
}

#[test]
#[traced_test]
async fn dispatch_deployment_verification_returns_ok() {
    let fixture = Fixture::new(function_name!()).await;

    let node = Node {
        node_id: "test-node-dv".to_string(),
        revision_id: "rev-test".to_string(),
        event: serde_json::json!({
            "type": "deployment_verification",
            "host_id": "antworker001",
            "service_id": "ant-host-agent",
            "environment": "beta",
        })
        .to_string(),
        state: "executable".to_string(),
        resource_key: None,
    };
    let d = Dispatch {
        direction: DispatchDirection::Deploy,
        revision_id: "rev-test".to_string(),
        node,
    };

    dispatch(fixture.state.clone(), d).await.unwrap();
}

#[test]
#[traced_test]
async fn dispatch_route_update_returns_ok() {
    let fixture = Fixture::new(function_name!()).await;

    let node = Node {
        node_id: "test-node-ru".to_string(),
        revision_id: "rev-test".to_string(),
        event: serde_json::json!({
            "type": "route_update",
            "environment": "beta",
        })
        .to_string(),
        state: "executable".to_string(),
        resource_key: None,
    };
    let d = Dispatch {
        direction: DispatchDirection::Deploy,
        revision_id: "rev-test".to_string(),
        node,
    };

    dispatch(fixture.state.clone(), d).await.unwrap();
}

#[test]
#[traced_test]
async fn dispatch_alert_configuration_returns_ok() {
    let fixture = Fixture::new(function_name!()).await;

    let node = Node {
        node_id: "test-node-ac".to_string(),
        revision_id: "rev-test".to_string(),
        event: serde_json::json!({
            "type": "alert_configuration",
            "service_id": "ant-host-agent",
            "environment": "beta",
        })
        .to_string(),
        state: "executable".to_string(),
        resource_key: None,
    };
    let d = Dispatch {
        direction: DispatchDirection::Deploy,
        revision_id: "rev-test".to_string(),
        node,
    };

    dispatch(fixture.state.clone(), d).await.unwrap();
}

#[test]
#[traced_test]
async fn dispatch_log_rule_configuration_returns_ok() {
    let fixture = Fixture::new(function_name!()).await;

    let node = Node {
        node_id: "test-node-lrc".to_string(),
        revision_id: "rev-test".to_string(),
        event: serde_json::json!({
            "type": "log_rule_configuration",
            "host_id": "antworker001",
            "service_id": "ant-host-agent",
        })
        .to_string(),
        state: "executable".to_string(),
        resource_key: None,
    };
    let d = Dispatch {
        direction: DispatchDirection::Deploy,
        revision_id: "rev-test".to_string(),
        node,
    };

    dispatch(fixture.state.clone(), d).await.unwrap();
}

#[test]
#[traced_test]
async fn dispatch_database_migration_returns_ok() {
    let fixture = Fixture::new(function_name!()).await;

    let node = Node {
        node_id: "test-node-dm".to_string(),
        revision_id: "rev-test".to_string(),
        event: serde_json::json!({
            "type": "database_migration",
            "service_id": "ant-host-agent",
            "environment": "beta",
        })
        .to_string(),
        state: "executable".to_string(),
        resource_key: None,
    };
    let d = Dispatch {
        direction: DispatchDirection::Deploy,
        revision_id: "rev-test".to_string(),
        node,
    };

    dispatch(fixture.state.clone(), d).await.unwrap();
}
