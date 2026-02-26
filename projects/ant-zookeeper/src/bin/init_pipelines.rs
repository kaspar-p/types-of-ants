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

    make_ant_data_farm_pipeline(&client).await?;
    make_ant_gateway_pipeline(&client).await?;
    make_website_pipeline(&client).await?;
    make_agent_pipeline(&client).await?;
    make_dynamic_dns_pipeline(&client).await?;

    Ok(())
}

async fn make_ant_gateway_pipeline(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let gateway_beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-gateway/beta".to_string(),
                project: "ant-gateway".to_string(),
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
            name: "ant-gateway".to_string(),
            stages: vec![vec![PutPipelineStage {
                name: "beta".to_string(),
                host_group_ids: vec![gateway_beta_hg_id],
            }]],
        })
        .await?;

    info!("Done: ant-gateway");

    Ok(())
}

async fn make_dynamic_dns_pipeline(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-naming-domains/beta".to_string(),
                project: "ant-naming-domains".to_string(),
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
            name: "ant-naming-domains".to_string(),
            stages: vec![vec![PutPipelineStage {
                name: "beta".to_string(),
                host_group_ids: vec![beta_hg_id],
            }]],
        })
        .await?;

    info!("Done: ant-naming-domains");

    Ok(())
}

async fn make_ant_data_farm_pipeline(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let data_farm_beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-data-farm/beta".to_string(),
                project: "ant-data-farm".to_string(),
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
            name: "ant-data-farm".to_string(),
            stages: vec![vec![PutPipelineStage {
                name: "beta".to_string(),
                host_group_ids: vec![data_farm_beta_hg_id],
            }]],
        })
        .await?;

    info!("Done: ant-data-farm");

    Ok(())
}

async fn make_website_pipeline(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let backend_beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-on-the-web/beta".to_string(),
                project: "ant-on-the-web".to_string(),
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

    let frontend_beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-looking-pretty/beta".to_string(),
                project: "ant-looking-pretty".to_string(),
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
            name: "website".to_string(),
            stages: vec![vec![PutPipelineStage {
                name: "beta-website".to_string(),
                host_group_ids: vec![backend_beta_hg_id, frontend_beta_hg_id],
            }]],
        })
        .await?;

    info!("Done: website");

    Ok(())
}

async fn make_agent_pipeline(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-host-agent/beta".to_string(),
                project: "ant-host-agent".to_string(),
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
                project: "ant-host-agent".to_string(),
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
            name: "agent".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_ids: vec![beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod-wave1".to_string(),
                    host_group_ids: vec![prod_wave1_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: agent");

    Ok(())
}
