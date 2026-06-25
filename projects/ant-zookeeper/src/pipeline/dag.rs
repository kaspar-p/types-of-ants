use crate::pipeline::deployment_event::DeploymentEvent;
use crate::pipeline::resource_key::{DeploymentResource, Identifier};
use crate::pipeline_engine::engine::PipelineEngine;
use crate::pipeline_engine::node::NodeOptions;

pub struct ProjectConfig {
    pub project_id: String,
    pub has_database: bool,
    pub has_routes: bool,
    pub has_alerts: bool,
    pub has_log_rules: bool,
    pub beta_hosts: Vec<String>,
    pub prod_hosts: Vec<String>,
}

fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}

fn node(event: DeploymentEvent, resource: Option<DeploymentResource>) -> NodeOptions {
    NodeOptions {
        event: serde_json::to_string(&event).unwrap(),
        mutates: resource.map(|r| r.to_string()),
    }
}

fn wave_layout(hosts: &[String]) -> Vec<Vec<&String>> {
    let sizes = [1, 2, 4];
    let mut waves = vec![];
    let mut cursor = 0;
    let mut size_idx = 0;

    loop {
        let chunk_size = if size_idx < sizes.len() {
            sizes[size_idx]
        } else {
            4
        };

        let mut wave = vec![];
        for _ in 0..chunk_size {
            if cursor >= hosts.len() {
                if !wave.is_empty() {
                    waves.push(wave);
                }
                return waves;
            }
            wave.push(&hosts[cursor]);
            cursor += 1;
        }

        waves.push(wave);
        size_idx += 1;
    }
}


pub async fn build_dag(
    engine: &PipelineEngine,
    revision_id: &str,
    config: &ProjectConfig,
) -> Result<String, anyhow::Error> {
    let pipeline_id = engine.create_pipeline(&config.project_id, revision_id).await?;

    let mut beta_terminal_nodes = vec![];

    if !config.beta_hosts.is_empty() {
        beta_terminal_nodes =
            build_env_dag(engine, &pipeline_id, config, "beta", &config.beta_hosts, vec![]).await?;
    }

    if !config.prod_hosts.is_empty() {
        build_env_dag(
            engine,
            &pipeline_id,
            config,
            "prod",
            &config.prod_hosts,
            beta_terminal_nodes,
        )
        .await?;
    }

    engine.seal(&pipeline_id).await?;

    Ok(pipeline_id)
}

async fn build_env_dag(
    engine: &PipelineEngine,
    pipeline_id: &str,
    config: &ProjectConfig,
    environment: &str,
    hosts: &[String],
    previous_node_ids: Vec<String>,
) -> Result<Vec<String>, anyhow::Error> {
    let mut chain_tip: Vec<String> = previous_node_ids;

    if config.has_routes {
        let n = engine
            .add_node(
                pipeline_id,
                node(
                    DeploymentEvent::RouteUpdate {
                        environment: environment.to_string(),
                    },
                    Some(DeploymentResource::GatewayRouting {
                        environment: id(environment),
                    }),
                ),
            )
            .await?;
        for prev in &chain_tip {
            engine.add_edge(prev, &n).await?;
        }
        chain_tip = vec![n];
    }

    if config.has_alerts {
        let n = engine
            .add_node(
                pipeline_id,
                node(
                    DeploymentEvent::AlertConfiguration {
                        service_id: config.project_id.clone(),
                        environment: environment.to_string(),
                    },
                    Some(DeploymentResource::AlertRules {
                        service_id: id(&config.project_id),
                    }),
                ),
            )
            .await?;
        for prev in &chain_tip {
            engine.add_edge(prev, &n).await?;
        }
        chain_tip = vec![n];
    }

    if config.has_log_rules {
        let mut log_rule_nodes = vec![];
        for host in hosts {
            let n = engine
                .add_node(
                    pipeline_id,
                    node(
                        DeploymentEvent::LogRuleConfiguration {
                            host_id: host.clone(),
                            service_id: config.project_id.clone(),
                        },
                        Some(DeploymentResource::LogRules {
                            host_id: id(host),
                            service_id: id(&config.project_id),
                        }),
                    ),
                )
                .await?;
            for prev in &chain_tip {
                engine.add_edge(prev, &n).await?;
            }
            log_rule_nodes.push(n);
        }
        chain_tip = log_rule_nodes;
    }

    if config.has_database {
        let n = engine
            .add_node(
                pipeline_id,
                node(
                    DeploymentEvent::DatabaseMigration {
                        service_id: config.project_id.clone(),
                        environment: environment.to_string(),
                    },
                    Some(DeploymentResource::DatabaseMigration {
                        service_id: id(&config.project_id),
                        environment: id(environment),
                    }),
                ),
            )
            .await?;
        for prev in &chain_tip {
            engine.add_edge(prev, &n).await?;
        }
        chain_tip = vec![n];
    }

    let waves = wave_layout(hosts);

    for wave in waves {
        let mut wave_terminal_nodes = vec![];

        for host in wave {
            let resource = Some(DeploymentResource::HostService {
                host_id: id(host),
                service_id: id(&config.project_id),
            });

            let replicate = engine
                .add_node(
                    pipeline_id,
                    node(
                        DeploymentEvent::ArtifactReplication {
                            host_id: host.clone(),
                            service_id: config.project_id.clone(),
                            environment: environment.to_string(),
                        },
                        resource.clone(),
                    ),
                )
                .await?;

            let deploy = engine
                .add_node(
                    pipeline_id,
                    node(
                        DeploymentEvent::HostDeployment {
                            host_id: host.clone(),
                            service_id: config.project_id.clone(),
                            environment: environment.to_string(),
                        },
                        resource.clone(),
                    ),
                )
                .await?;

            let verify = engine
                .add_node(
                    pipeline_id,
                    node(
                        DeploymentEvent::DeploymentVerification {
                            host_id: host.clone(),
                            service_id: config.project_id.clone(),
                            environment: environment.to_string(),
                        },
                        resource,
                    ),
                )
                .await?;

            for prev in &chain_tip {
                engine.add_edge(prev, &replicate).await?;
            }
            engine.add_edge(&replicate, &deploy).await?;
            engine.add_edge(&deploy, &verify).await?;

            wave_terminal_nodes.push(verify);
        }

        chain_tip = wave_terminal_nodes;
    }

    Ok(chain_tip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ant_library::db::TypesOfAntsDatabase;
    use ant_library_test::db::TestDatabase;
    use ant_zookeeper_db::AntZooStorageClient;
    use tracing_test::traced_test;

    struct Fixture {
        engine: PipelineEngine,
        db: AntZooStorageClient,
        _guard: TestDatabase,
    }

    impl Fixture {
        async fn new() -> Self {
            let guard = TestDatabase::new("ant-zookeeper-db").await;
            let db = AntZooStorageClient::connect(&guard.config).await.unwrap();
            let engine = PipelineEngine::new(db.pool()).await.unwrap();
            Fixture { engine, db, _guard: guard }
        }
    }

    fn events(nodes: &[crate::pipeline_engine::engine::Node]) -> Vec<DeploymentEvent> {
        nodes
            .iter()
            .map(|n| serde_json::from_str(&n.event).unwrap())
            .collect()
    }

    #[tokio::test]
    #[traced_test]
    async fn dag_simple_binary_beta_and_prod() {
        let f = Fixture::new().await;
        let rev = f.db.create_revision("ant-on-the-web").await.unwrap();

        let config = ProjectConfig {
            project_id: "ant-on-the-web".to_string(),
            has_database: false,
            has_routes: false,
            has_alerts: false,
            has_log_rules: false,
            beta_hosts: vec!["w2".to_string()],
            prod_hosts: vec!["w1".to_string(), "w3".to_string(), "w4".to_string()],
        };

        let pipeline_id = build_dag(&f.engine, &rev, &config).await.unwrap();

        let nodes = f.engine.nodes(&pipeline_id).await.unwrap();
        let edges = f.engine.edges(&pipeline_id).await.unwrap();

        // Beta: 1 host × 3 nodes = 3
        // Prod: 3 hosts in waves [1, 2] × 3 nodes = 9
        // Total: 12 nodes
        assert_eq!(nodes.len(), 12);

        // Beta triplet is a chain
        let beta_events: Vec<_> = events(&nodes)
            .into_iter()
            .filter(|e| match e {
                DeploymentEvent::ArtifactReplication { host_id, .. }
                | DeploymentEvent::HostDeployment { host_id, .. }
                | DeploymentEvent::DeploymentVerification { host_id, .. } => host_id == "w2",
                _ => false,
            })
            .collect();
        assert_eq!(beta_events.len(), 3);

        // Prod wave 1 has 1 host, wave 2 has 2 hosts
        let prod_events: Vec<_> = events(&nodes)
            .into_iter()
            .filter(|e| match e {
                DeploymentEvent::ArtifactReplication { host_id, .. }
                | DeploymentEvent::HostDeployment { host_id, .. }
                | DeploymentEvent::DeploymentVerification { host_id, .. } => host_id != "w2",
                _ => false,
            })
            .collect();
        assert_eq!(prod_events.len(), 9);

        // Edges: beta has 2 internal + prod wave structure
        assert!(edges.len() > 0);

        // Beta verify → prod wave 1 replicate (sequential environments)
        let beta_verify_id = nodes.iter().find(|n| {
            n.event.contains("deployment_verification") && n.event.contains("w2")
        }).unwrap();
        let prod_first_replicate = nodes.iter().find(|n| {
            n.event.contains("artifact_replication") && n.event.contains("w1")
        }).unwrap();
        assert!(edges.iter().any(|e| {
            e.from_node_id == beta_verify_id.node_id && e.to_node_id == prod_first_replicate.node_id
        }));
    }

    #[tokio::test]
    #[traced_test]
    async fn dag_with_database_migration() {
        let f = Fixture::new().await;
        let rev = f.db.create_revision("ant-data-farm").await.unwrap();

        let config = ProjectConfig {
            project_id: "ant-data-farm".to_string(),
            has_database: true,
            has_routes: false,
            has_alerts: false,
            has_log_rules: false,
            beta_hosts: vec!["w2".to_string()],
            prod_hosts: vec!["w1".to_string()],
        };

        let pipeline_id = build_dag(&f.engine, &rev, &config).await.unwrap();

        let nodes = f.engine.nodes(&pipeline_id).await.unwrap();
        let edges = f.engine.edges(&pipeline_id).await.unwrap();

        let node_events = events(&nodes);

        // Should have 2 migration nodes (beta + prod)
        let migrations: Vec<_> = node_events
            .iter()
            .filter(|e| matches!(e, DeploymentEvent::DatabaseMigration { .. }))
            .collect();
        assert_eq!(migrations.len(), 2);

        // Migration should come before host deployment in each env
        let beta_migration = nodes.iter().find(|n| {
            n.event.contains("database_migration") && n.event.contains("beta")
        }).unwrap();
        let beta_replicate = nodes.iter().find(|n| {
            n.event.contains("artifact_replication") && n.event.contains("w2")
        }).unwrap();
        assert!(edges.iter().any(|e| {
            e.from_node_id == beta_migration.node_id && e.to_node_id == beta_replicate.node_id
        }));
    }

    #[tokio::test]
    #[traced_test]
    async fn dag_with_all_config_and_database() {
        let f = Fixture::new().await;
        let rev = f.db.create_revision("ant-on-the-web").await.unwrap();

        let config = ProjectConfig {
            project_id: "ant-on-the-web".to_string(),
            has_database: true,
            has_routes: true,
            has_alerts: true,
            has_log_rules: true,
            beta_hosts: vec!["w2".to_string()],
            prod_hosts: vec!["w1".to_string()],
        };

        let pipeline_id = build_dag(&f.engine, &rev, &config).await.unwrap();

        let nodes = f.engine.nodes(&pipeline_id).await.unwrap();
        let edges = f.engine.edges(&pipeline_id).await.unwrap();
        let node_events = events(&nodes);

        // Per env: route_update → alert_config → log_rule_config → db_migration → host triplet
        // Beta: 4 config + 3 host = 7
        // Prod: 4 config + 3 host = 7
        // Total: 14
        assert_eq!(nodes.len(), 14);

        // Verify serial ordering in beta: route → alerts → log_rules → migration → replicate
        let beta_route = nodes.iter().find(|n| {
            n.event.contains("route_update") && n.event.contains("beta")
        }).unwrap();
        let alerts = nodes.iter().find(|n| {
            n.event.contains("alert_configuration")
        }).unwrap();

        assert!(edges.iter().any(|e| {
            e.from_node_id == beta_route.node_id && e.to_node_id == alerts.node_id
        }));
    }

    #[tokio::test]
    #[traced_test]
    async fn dag_prod_only_no_beta() {
        let f = Fixture::new().await;
        let rev = f.db.create_revision("ant-gateway").await.unwrap();

        let config = ProjectConfig {
            project_id: "ant-gateway".to_string(),
            has_database: false,
            has_routes: true,
            has_alerts: false,
            has_log_rules: false,
            beta_hosts: vec![],
            prod_hosts: vec!["w1".to_string()],
        };

        let pipeline_id = build_dag(&f.engine, &rev, &config).await.unwrap();

        let nodes = f.engine.nodes(&pipeline_id).await.unwrap();

        // route_update(prod) + 3 host nodes = 4
        assert_eq!(nodes.len(), 4);

        // First node should be route update (root after seal)
        let route_node = nodes.iter().find(|n| n.event.contains("route_update")).unwrap();
        assert_eq!(route_node.state, "executable");
    }
}

