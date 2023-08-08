use ant_owning_artifacts::procs::deploy_project::{deploy_project, DeployProjectRequest};
use axum::Json;

#[tracing_test::traced_test()]
#[tokio::test]
async fn build_ant_who_tweets() {
    let req = Json(DeployProjectRequest {
        project: ant_metadata::Project::AntWhoTweets,
        architecture: ant_metadata::Architecture::RaspberryPi,
        selection: ant_metadata::ArtifactSelection::Latest,
    });
    let res = deploy_project(req).await.unwrap();
    assert!(res.0.host.label.contains("Raspberry"));
    assert!(res.0.host.label.contains("Pi"));
}
