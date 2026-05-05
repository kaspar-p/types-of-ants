use ant_zookeeper::{
    client::{AntZookeeperClient, AntZookeeperClientConfig},
    routes::pipeline::{
        AddHostToHostGroupRequest, CreateHostGroupRequest, PutPipelineRequest, PutPipelineStage,
    },
};

use ant_library::services::{ServiceEnv, Services};

pub fn find_up(filename: &str) -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap();

    loop {
        let candidate = dir.join(filename);
        if std::fs::exists(&candidate).unwrap() {
            return candidate;
        }

        dir = dir
            .parent()
            .expect(&format!("got to root without finding: {filename}"))
            .to_path_buf()
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    let services: Services =
        serde_json::de::from_reader(std::fs::File::open(find_up("services.json")).unwrap())
            .unwrap();
    services.validate().expect("malformed services.json");

    let client = AntZookeeperClient::new(AntZookeeperClientConfig {
        tls: false,
        endpoint: "localhost:3235".to_string(),
    });

    for service_id in services.list_service_ids() {
        let hosts = services.list_hosts_with_service(&service_id);

        let beta_hg_id = if hosts.iter().any(|(_, s)| matches!(s.env, ServiceEnv::Beta)) {
            let hg = client
                .create_host_group(CreateHostGroupRequest {
                    name: format!("{service_id}/beta"),
                    project: service_id.to_string(),
                    environment: "beta".to_string(),
                })
                .await?;

            for (host_id, _) in hosts.iter().filter(|h| matches!(h.1.env, ServiceEnv::Beta)) {
                client
                    .add_host_to_host_group(AddHostToHostGroupRequest {
                        host_group_id: hg.id.clone(),
                        host_id: host_id.to_string(),
                    })
                    .await?;
            }

            Some(hg.id)
        } else {
            None
        };

        let prod_hg_id = if hosts.iter().any(|(_, s)| matches!(s.env, ServiceEnv::Prod)) {
            let hg = client
                .create_host_group(CreateHostGroupRequest {
                    name: format!("{service_id}/prod"),
                    project: service_id.to_string(),
                    environment: "prod".to_string(),
                })
                .await?;

            for (host_id, _) in hosts.iter().filter(|h| matches!(h.1.env, ServiceEnv::Prod)) {
                client
                    .add_host_to_host_group(AddHostToHostGroupRequest {
                        host_group_id: hg.id.clone(),
                        host_id: host_id.to_string(),
                    })
                    .await?;
            }

            Some(hg.id)
        } else {
            None
        };

        let beta_stage = beta_hg_id.map(|id| PutPipelineStage {
            name: "beta".to_string(),
            host_group_ids: vec![id],
        });
        let prod_stage = prod_hg_id.map(|id| PutPipelineStage {
            name: "prod".to_string(),
            host_group_ids: vec![id],
        });

        client
            .put_pipeline(PutPipelineRequest {
                name: service_id.to_string(),
                stages: vec![beta_stage, prod_stage]
                    .into_iter()
                    .filter_map(|s| s)
                    .map(|s| vec![s])
                    .collect(),
            })
            .await?;

        println!("Done: {service_id}")
    }

    // pipeline_ant_data_farm(&client).await?;
    // pipeline_ant_host_agent(&client).await?;
    // pipeline_ant_who_tweets(&client).await?;
    // pipeline_ant_backing_it_up_and_ant_backing_it_up_db(&client).await?;
    // pipeline_ant_gateway(&client).await?;
    // pipeline_ant_on_the_web_and_ant_looking_pretty(&client).await?;
    // pipeline_ant_naming_domains(&client).await?;
    // pipeline_ant_fs(&client).await?;

    // pipeline_ant_monitor(&client).await?;
    // pipeline_ant_worker_metrics_exporter(&client).await?;

    Ok(())
}

// async fn pipeline_ant_backing_it_up_and_ant_backing_it_up_db(
//     client: &AntZookeeperClient,
// ) -> Result<(), anyhow::Error> {
//     let ws_beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-backing-it-up/beta".to_string(),
//                 project: "ant-backing-it-up".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let db_beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-backing-it-up-db/beta".to_string(),
//                 project: "ant-backing-it-up-db".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let db_prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-backing-it-up-db/prod".to_string(),
//                 project: "ant-backing-it-up-db".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker003.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let ws_prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-backing-it-up/prod".to_string(),
//                 project: "ant-backing-it-up".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker001.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "ant-backing-it-up".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "beta".to_string(),
//                     host_group_ids: vec![db_beta_hg_id, ws_beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "prod".to_string(),
//                     host_group_ids: vec![db_prod_hg_id, ws_prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: ant-backing-it-up");

//     Ok(())
// }

// async fn pipeline_ant_gateway(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
//     let gateway_beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-gateway/beta".to_string(),
//                 project: "ant-gateway".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let gateway_prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-gateway/prod".to_string(),
//                 project: "ant-gateway".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker001.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "ant-gateway".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "beta".to_string(),
//                     host_group_ids: vec![gateway_beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "prod".to_string(),
//                     host_group_ids: vec![gateway_prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: ant-gateway");

//     Ok(())
// }

// async fn pipeline_ant_who_tweets(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
//     let beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-who-tweets/beta".to_string(),
//                 project: "ant-who-tweets".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-who-tweets/prod".to_string(),
//                 project: "ant-who-tweets".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker005.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "ant-who-tweets".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "@beta-typesofants".to_string(),
//                     host_group_ids: vec![beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "@typesofants".to_string(),
//                     host_group_ids: vec![prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: ant-who-tweets");

//     Ok(())
// }

// async fn pipeline_ant_naming_domains(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
//     let beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-naming-domains/beta".to_string(),
//                 project: "ant-naming-domains".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-naming-domains/prod".to_string(),
//                 project: "ant-naming-domains".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker001.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "ant-naming-domains".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "beta".to_string(),
//                     host_group_ids: vec![beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "prod".to_string(),
//                     host_group_ids: vec![prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: ant-naming-domains");

//     Ok(())
// }

// async fn pipeline_ant_data_farm(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
//     let data_farm_beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-data-farm/beta".to_string(),
//                 project: "ant-data-farm".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let data_farm_prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-data-farm/prod".to_string(),
//                 project: "ant-data-farm".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker006.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "ant-data-farm".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "beta".to_string(),
//                     host_group_ids: vec![data_farm_beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "prod".to_string(),
//                     host_group_ids: vec![data_farm_prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: ant-data-farm");

//     Ok(())
// }

// async fn pipeline_ant_on_the_web_and_ant_looking_pretty(
//     client: &AntZookeeperClient,
// ) -> Result<(), anyhow::Error> {
//     let backend_beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-on-the-web/beta".to_string(),
//                 project: "ant-on-the-web".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let frontend_beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-looking-pretty/beta".to_string(),
//                 project: "ant-looking-pretty".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let backend_prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-on-the-web/prod".to_string(),
//                 project: "ant-on-the-web".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker001.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let frontend_prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-looking-pretty/prod".to_string(),
//                 project: "ant-looking-pretty".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker001.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "website".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "beta-website".to_string(),
//                     host_group_ids: vec![backend_beta_hg_id, frontend_beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "prod-website".to_string(),
//                     host_group_ids: vec![backend_prod_hg_id, frontend_prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: website");

//     Ok(())
// }

// async fn pipeline_ant_host_agent(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
//     let all_hostgroup_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-host-agent/all".to_string(),
//                 project: "ant-host-agent".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         for host in [
//             "antworker001.hosts.typesofants.org",
//             "antworker002.hosts.typesofants.org",
//             "antworker003.hosts.typesofants.org",
//             "antworker004.hosts.typesofants.org",
//             "antworker005.hosts.typesofants.org",
//             "antworker006.hosts.typesofants.org",
//             "antworker007.hosts.typesofants.org",
//         ] {
//             client
//                 .add_host_to_host_group(AddHostToHostGroupRequest {
//                     host_group_id: hg.id.clone(),
//                     host_id: host.to_string(),
//                 })
//                 .await?;
//         }

//         hg.id
//     };

//     // let prod_wave1_hg_id = {
//     //     let hg = client
//     //         .create_host_group(CreateHostGroupRequest {
//     //             name: "ant-host-agent/prod-wave1".to_string(),
//     //             project: "ant-host-agent".to_string(),
//     //             environment: "prod".to_string(),
//     //         })
//     //         .await?;

//     //     client
//     //         .add_host_to_host_group(AddHostToHostGroupRequest {
//     //             host_group_id: hg.id.clone(),
//     //             host_id: "antworker001.hosts.typesofants.org".to_string(),
//     //         })
//     //         .await?;

//     //     hg.id
//     // };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "agent".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "all-agents".to_string(),
//                     host_group_ids: vec![all_hostgroup_id],
//                 }],
//                 // vec![PutPipelineStage {
//                 //     name: "prod-wave1".to_string(),
//                 //     host_group_ids: vec![prod_wave1_hg_id],
//                 // }],
//             ],
//         })
//         .await?;

//     info!("Done: agent");

//     Ok(())
// }

// async fn pipeline_ant_fs(client: &AntZookeeperClient) -> Result<(), anyhow::Error> {
//     let beta_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-fs/beta".to_string(),
//                 project: "ant-fs".to_string(),
//                 environment: "beta".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker002.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     let prod_hg_id = {
//         let hg = client
//             .create_host_group(CreateHostGroupRequest {
//                 name: "ant-fs/prod".to_string(),
//                 project: "ant-fs".to_string(),
//                 environment: "prod".to_string(),
//             })
//             .await?;

//         client
//             .add_host_to_host_group(AddHostToHostGroupRequest {
//                 host_group_id: hg.id.clone(),
//                 host_id: "antworker004.hosts.typesofants.org".to_string(),
//             })
//             .await?;

//         hg.id
//     };

//     client
//         .put_pipeline(PutPipelineRequest {
//             name: "ant-fs".to_string(),
//             stages: vec![
//                 vec![PutPipelineStage {
//                     name: "beta".to_string(),
//                     host_group_ids: vec![beta_hg_id],
//                 }],
//                 vec![PutPipelineStage {
//                     name: "prod".to_string(),
//                     host_group_ids: vec![prod_hg_id],
//                 }],
//             ],
//         })
//         .await?;

//     info!("Done: ant-fs");

//     Ok(())
// }
