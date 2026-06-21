use ant_zookeeper::{client::AntZookeeperClientConfig, routes::service::UpsertRevisionRequest};
use clap_complete::ArgValueCompleter;
use tracing::info;

use crate::{cmd::build::BuildCmd, complete::complete_projects};

#[derive(Clone, clap::Args)]
pub struct DeployCmd {
    #[arg(add = ArgValueCompleter::new(complete_projects))]
    project: String,
}

pub async fn deploy(cmd: DeployCmd) -> () {
    let client = ant_zookeeper::client::AntZookeeperClient::new(AntZookeeperClientConfig {
        tls: false,
        endpoint: "localhost:3235".to_string(),
    });

    let revision = client
        .upsert_revision(UpsertRevisionRequest {
            project: cmd.project.clone(),
        })
        .await
        .unwrap();

    let files = crate::cmd::build::build(BuildCmd::new(cmd.project.clone(), None)).await;

    let handles = files
        .into_iter()
        .map(|f| {
            let project = cmd.project.clone();
            let rev2 = revision.revision.clone();
            tokio::task::spawn_blocking(|| async move {
                info!("... registering artifact");
                let client =
                    ant_zookeeper::client::AntZookeeperClient::new(AntZookeeperClientConfig {
                        tls: false,
                        endpoint: "localhost:3235".to_string(),
                    });
                client
                    .register_artifact(&rev2, &project, &f.arch, &f.version, &f.file.file_path())
                    .await
                    .expect("register artifact");

                f.file.close().expect("failed to close deployment file");

                info!("... artifact registered.");
            })
        })
        .collect::<Vec<_>>();

    let handles = futures::future::join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("task scheduling failed");

    futures::future::join_all(handles).await;

    ()
}
