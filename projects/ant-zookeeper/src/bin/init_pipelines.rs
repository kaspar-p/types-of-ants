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

    pipeline_ant_data_farm(&client).await?;
    pipeline_ant_host_agent(&client).await?;
    pipeline_ant_who_tweets(&client).await?;
    pipeline_ant_backing_it_up_and_ant_backing_it_up_db(&client).await?;
    pipeline_ant_gateway(&client).await?;
    pipeline_ant_on_the_web_and_ant_looking_pretty(&client).await?;
    pipeline_ant_naming_domains(&client).await?;
    pipeline_ant_fs(&client).await?;

    Ok(())
}

async fn pipeline_ant_backing_it_up_and_ant_backing_it_up_db(
    client: &AntZookeeperClient,
) -> Result<(), anyhow::Error> {
    let ws_beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-backing-it-up/beta".to_string(),
                project: "ant-backing-it-up".to_string(),
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

    let db_beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-backing-it-up-db/beta".to_string(),
                project: "ant-backing-it-up-db".to_string(),
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

    let db_prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-backing-it-up-db/prod".to_string(),
                project: "ant-backing-it-up-db".to_string(),
                environment: "prod".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker003.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    let ws_prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-backing-it-up/prod".to_string(),
                project: "ant-backing-it-up".to_string(),
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
            name: "ant-backing-it-up".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_ids: vec![db_beta_hg_id, ws_beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod".to_string(),
                    host_group_ids: vec![db_prod_hg_id, ws_prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: ant-backing-it-up");

    Ok(())
}

async fn pipeline_ant_gateway(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
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

    let gateway_prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-gateway/prod".to_string(),
                project: "ant-gateway".to_string(),
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
            name: "ant-gateway".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_ids: vec![gateway_beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod".to_string(),
                    host_group_ids: vec![gateway_prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: ant-gateway");

    Ok(())
}

async fn pipeline_ant_who_tweets(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-who-tweets/beta".to_string(),
                project: "ant-who-tweets".to_string(),
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

    let prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-who-tweets/prod".to_string(),
                project: "ant-who-tweets".to_string(),
                environment: "prod".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker005.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            name: "ant-who-tweets".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "@beta-typesofants".to_string(),
                    host_group_ids: vec![beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "@typesofants".to_string(),
                    host_group_ids: vec![prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: ant-who-tweets");

    Ok(())
}

async fn pipeline_ant_naming_domains(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
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

    let prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-naming-domains/prod".to_string(),
                project: "ant-naming-domains".to_string(),
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
            name: "ant-naming-domains".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_ids: vec![beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod".to_string(),
                    host_group_ids: vec![prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: ant-naming-domains");

    Ok(())
}

async fn pipeline_ant_data_farm(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
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

    let data_farm_prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-data-farm/prod".to_string(),
                project: "ant-data-farm".to_string(),
                environment: "prod".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker006.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            name: "ant-data-farm".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_ids: vec![data_farm_beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod".to_string(),
                    host_group_ids: vec![data_farm_prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: ant-data-farm");

    Ok(())
}

async fn pipeline_ant_on_the_web_and_ant_looking_pretty(
    client: &AntZookeeperClient,
) -> Result<(), anyhow::Error> {
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

    let backend_prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-on-the-web/prod".to_string(),
                project: "ant-on-the-web".to_string(),
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

    let frontend_prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-looking-pretty/prod".to_string(),
                project: "ant-looking-pretty".to_string(),
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
            name: "website".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta-website".to_string(),
                    host_group_ids: vec![backend_beta_hg_id, frontend_beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod-website".to_string(),
                    host_group_ids: vec![backend_prod_hg_id, frontend_prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: website");

    Ok(())
}

async fn pipeline_ant_host_agent(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let all_hostgroup_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-host-agent/all".to_string(),
                project: "ant-host-agent".to_string(),
                environment: "prod".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker003.hosts.typesofants.org".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker005.hosts.typesofants.org".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker006.hosts.typesofants.org".to_string(),
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

    // let prod_wave1_hg_id = {
    //     let hg = client
    //         .create_host_group(CreateHostGroupRequest {
    //             name: "ant-host-agent/prod-wave1".to_string(),
    //             project: "ant-host-agent".to_string(),
    //             environment: "prod".to_string(),
    //         })
    //         .await?;

    //     client
    //         .add_host_to_host_group(AddHostToHostGroupRequest {
    //             host_group_id: hg.id.clone(),
    //             host_id: "antworker001.hosts.typesofants.org".to_string(),
    //         })
    //         .await?;

    //     hg.id
    // };

    client
        .put_pipeline(PutPipelineRequest {
            name: "agent".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "all-agents".to_string(),
                    host_group_ids: vec![all_hostgroup_id],
                }],
                // vec![PutPipelineStage {
                //     name: "prod-wave1".to_string(),
                //     host_group_ids: vec![prod_wave1_hg_id],
                // }],
            ],
        })
        .await?;

    info!("Done: agent");

    Ok(())
}

async fn pipeline_ant_fs(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
    let beta_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-fs/beta".to_string(),
                project: "ant-fs".to_string(),
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

    let prod_hg_id = {
        let hg = client
            .create_host_group(CreateHostGroupRequest {
                name: "ant-fs/prod".to_string(),
                project: "ant-fs".to_string(),
                environment: "prod".to_string(),
            })
            .await?;

        client
            .add_host_to_host_group(AddHostToHostGroupRequest {
                host_group_id: hg.id.clone(),
                host_id: "antworker004.hosts.typesofants.org".to_string(),
            })
            .await?;

        hg.id
    };

    client
        .put_pipeline(PutPipelineRequest {
            name: "ant-fs".to_string(),
            stages: vec![
                vec![PutPipelineStage {
                    name: "beta".to_string(),
                    host_group_ids: vec![beta_hg_id],
                }],
                vec![PutPipelineStage {
                    name: "prod".to_string(),
                    host_group_ids: vec![prod_hg_id],
                }],
            ],
        })
        .await?;

    info!("Done: ant-fs");

    Ok(())
}
