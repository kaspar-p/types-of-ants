use std::fs::{self, File};
use std::io::Write;

use crate::fixture::{self, Fixture};
use ant_zookeeper::routes::projects::ProjectDeploymentView;
use ant_zookeeper::routes::service::{UpsertRevisionRequest, UpsertRevisionResponse};
use http::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

fn setup_gateway_secrets(fixture: &Fixture) {
    let secret_root = fixture.state.root_dir.join("secrets-db");
    for path in [
        secret_root.join("beta").join("tls_cert.secret"),
        secret_root.join("prod").join("tls_cert.secret"),
        secret_root.join("beta").join("tls_key.secret"),
        secret_root.join("prod").join("tls_key.secret"),
    ] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(path).unwrap().write_all(b"secret").unwrap();
    }
}

async fn upload_artifact(fixture: &Fixture, rev_id: &str, arch: &str, version: &str) {
    let tarfile = fixture.make_tarfile_fixture("ant-gateway-v1");
    let file_bytes = fs::read(tarfile.path()).unwrap();

    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(file_bytes).file_name("artifact.tar.gz"),
    );

    let res = fixture
        .client
        .post("/service/artifact")
        .header("X-Ant-Revision", rev_id)
        .header("X-Ant-Project", "ant-gateway")
        .header("X-Ant-Architecture", arch)
        .header("X-Ant-Version", version)
        .multipart(form)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}

async fn get_view(fixture: &Fixture) -> ProjectDeploymentView {
    let res = fixture.client.get("/projects/ant-gateway").send().await;
    assert_eq!(res.status(), StatusCode::OK);
    res.json().await
}

#[tokio::test]
#[traced_test]
async fn projects_returns_200_no_activity() {
    let fixture = Fixture::new(function_name!()).await;

    let body = get_view(&fixture).await;
    assert!(body.build.is_none());
    assert!(body.active_pipelines.is_empty());
    assert!(body.latest_finished_pipeline.is_none());
}

#[tokio::test]
#[traced_test]
async fn projects_returns_200_build_in_progress() {
    let fixture = Fixture::new(function_name!()).await;
    setup_gateway_secrets(&fixture);

    let res = fixture
        .client
        .post("/service/revision")
        .json(&UpsertRevisionRequest {
            project: "ant-gateway".to_string(),
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let revision_resp: UpsertRevisionResponse = res.json().await;
    let rev_id = revision_resp.revision;

    upload_artifact(&fixture, &rev_id, "x86_64", "build-789").await;

    let body = get_view(&fixture).await;

    let build = body.build.unwrap();
    assert_eq!(build.revision_id, rev_id);
    assert_eq!(build.artifacts.len(), 1);
    assert_eq!(build.artifacts[0].architecture, "x86_64");
    assert_eq!(build.artifacts[0].build_version, "build-789");
    assert!(build.artifacts[0].size_bytes > 0);
    assert!(!build.artifacts[0].fingerprint.is_empty());
    assert_eq!(build.missing_architectures.len(), 2);

    assert!(body.active_pipelines.is_empty());
    assert!(body.latest_finished_pipeline.is_none());
}

#[tokio::test]
#[traced_test]
async fn projects_returns_200_build_activates_into_pipeline() {
    let fixture = Fixture::new(function_name!()).await;
    setup_gateway_secrets(&fixture);

    let res = fixture
        .client
        .post("/service/revision")
        .json(&UpsertRevisionRequest {
            project: "ant-gateway".to_string(),
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let revision_resp: UpsertRevisionResponse = res.json().await;
    let rev_id = revision_resp.revision;

    upload_artifact(&fixture, &rev_id, "x86_64", "build-100").await;
    upload_artifact(&fixture, &rev_id, "aarch64", "build-100").await;
    upload_artifact(&fixture, &rev_id, "armv7", "build-100").await;

    let body = get_view(&fixture).await;

    assert!(body.build.is_none());
    assert_eq!(body.active_pipelines.len(), 1);
    assert_eq!(body.active_pipelines[0].revision_id, rev_id);
    assert!(!body.active_pipelines[0].layers.is_empty());
    assert!(body.latest_finished_pipeline.is_none());
}

#[tokio::test]
#[traced_test]
async fn projects_returns_200_pipeline_finishes() {
    let fixture = Fixture::new(function_name!()).await;
    setup_gateway_secrets(&fixture);

    let res = fixture
        .client
        .post("/service/revision")
        .json(&UpsertRevisionRequest {
            project: "ant-gateway".to_string(),
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let revision_resp: UpsertRevisionResponse = res.json().await;
    let rev_id = revision_resp.revision;

    upload_artifact(&fixture, &rev_id, "x86_64", "build-200").await;
    upload_artifact(&fixture, &rev_id, "aarch64", "build-200").await;
    upload_artifact(&fixture, &rev_id, "armv7", "build-200").await;

    for _ in 0..20 {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);

        let body = get_view(&fixture).await;
        if body.latest_finished_pipeline.is_some() {
            assert!(body.active_pipelines.is_empty());
            assert_eq!(body.latest_finished_pipeline.unwrap().revision_id, rev_id);
            return;
        }
    }

    panic!("Pipeline did not finish within 20 ticks");
}

#[tokio::test]
#[traced_test]
async fn projects_returns_200_new_build_after_finished() {
    let fixture = Fixture::new(function_name!()).await;
    setup_gateway_secrets(&fixture);

    // First revision: build + activate + run to completion
    let res = fixture
        .client
        .post("/service/revision")
        .json(&UpsertRevisionRequest {
            project: "ant-gateway".to_string(),
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let rev1: UpsertRevisionResponse = res.json().await;

    upload_artifact(&fixture, &rev1.revision, "x86_64", "build-300").await;
    upload_artifact(&fixture, &rev1.revision, "aarch64", "build-300").await;
    upload_artifact(&fixture, &rev1.revision, "armv7", "build-300").await;

    for _ in 0..20 {
        let res = fixture.client.post("/deployment/iteration").send().await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    // Second revision: start building
    let res = fixture
        .client
        .post("/service/revision")
        .json(&UpsertRevisionRequest {
            project: "ant-gateway".to_string(),
        })
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let rev2: UpsertRevisionResponse = res.json().await;

    upload_artifact(&fixture, &rev2.revision, "x86_64", "build-400").await;

    let body = get_view(&fixture).await;

    let build = body.build.unwrap();
    assert_eq!(build.revision_id, rev2.revision);
    assert_eq!(build.artifacts.len(), 1);
    assert_eq!(build.missing_architectures.len(), 2);

    assert!(body.active_pipelines.is_empty());

    let finished = body.latest_finished_pipeline.unwrap();
    assert_eq!(finished.revision_id, rev1.revision);
}
