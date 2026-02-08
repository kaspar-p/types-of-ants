use ant_zookeeper::{
    client::{AntZookeeperClient, AntZookeeperClientConfig},
    routes::pipeline::{
        AddHostToHostGroupRequest, CreateHostGroupRequest, PutPipelineRequest, PutPipelineStage,
    },
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    let client = AntZookeeperClient::new(AntZookeeperClientConfig {
        tls: false,
        endpoint: "localhost:3235".to_string(),
    });

    make_ant_zookeeper_db(&client).await?;
    make_ant_on_the_web(&client).await?;
    make_ant_looking_pretty(&client).await?;
    make_ant_host_agent(&client).await?;
    make_ant_gateway(&client).await?;

    info!("Done!");

    Ok(())
}

async fn make_ant_gateway(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-gateway/beta".to_string(),
                environment: "beta".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker002.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            project: "ant-gateway".to_string(),
            stages: vec![PutPipelineStage {
                name: "beta-website".to_string(),
                host_group_id: beta_hg_id,
            }],
        })
        .await?;

    Ok(())
}

async fn make_ant_on_the_web(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-on-the-web/beta".to_string(),
                environment: "beta".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker002.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            project: "ant-on-the-web".to_string(),
            stages: vec![PutPipelineStage {
                name: "beta-website".to_string(),
                host_group_id: beta_hg_id,
            }],
        })
        .await?;

    Ok(())
}

async fn make_ant_looking_pretty(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-looking-pretty/beta".to_string(),
                environment: "beta".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker002.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            project: "ant-looking-pretty".to_string(),
            stages: vec![PutPipelineStage {
                name: "beta-website".to_string(),
                host_group_id: beta_hg_id,
            }],
        })
        .await?;

    Ok(())
}

async fn make_ant_host_agent(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-host-agent/beta".to_string(),
                environment: "beta".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker002.hosts.typesofants.org".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker007.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    let prod_wave1_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-host-agent/prod-wave1".to_string(),
                environment: "prod".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker001.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            project: "ant-host-agent".to_string(),
            stages: vec![
                PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_id: beta_hg_id,
                },
                PutPipelineStage {
                    name: "prod-wave1".to_string(),
                    host_group_id: prod_wave1_hg_id,
                },
            ],
        })
        .await?;

    Ok(())
}

async fn make_ant_zookeeper_db(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let hg = client
        .create_host_group(CreateHostGroupRequest {
            name: "ant-zookeeper-db/only".to_string(),
            environment: "prod".to_string(),
        })
        .await?;

    client
        .add_host_to_host_group(AddHostToHostGroupRequest {
            host_group_id: hg.id.clone(),
            host_id: "antworker007.hosts.typesofants.org".to_string(),
        })
        .await?;

    client
        .put_pipeline(PutPipelineRequest {
            project: "ant-zookeeper-db".to_string(),
            stages: vec![PutPipelineStage {
                name: "deploy".to_string(),
                host_group_id: hg.id.clone(),
            }],
        })
        .await?;

    Ok(())
}
