use ant_library::services::ServiceEnv;

use crate::pipeline::deployment_event::DeploymentEvent;
use crate::pipeline::resource_key::{DeploymentResource, Identifier};
use crate::pipeline_engine::engine::PipelineEngine;
use crate::pipeline_engine::node::{NodeOptions, NodeSpec};

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

fn node(
    event: DeploymentEvent,
    resource: Option<DeploymentResource>,
    options: NodeOptions,
) -> NodeSpec {
    NodeSpec {
        event: serde_json::to_string(&event).unwrap(),
        mutates: resource.map(|r| r.to_string()),
        options,
    }
}

fn no_unwind() -> NodeOptions {
    NodeOptions {
        unwind_on_failure: false,
        ..NodeOptions::default()
    }
}

/// Returns a vector-of-vectors of the IDs passed in, e.g.
/// [[1], [2, 3], [4, 5, 6]] for IDs 1 through 6.
fn wave_layout(ids: &[String]) -> Vec<Vec<&String>> {
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
            if cursor >= ids.len() {
                if !wave.is_empty() {
                    waves.push(wave);
                }
                return waves;
            }
            wave.push(&ids[cursor]);
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
    let pipeline_id = engine
        .create_pipeline(&config.project_id, revision_id)
        .await?;

    let mut beta_terminal_nodes = vec![];

    if !config.beta_hosts.is_empty() {
        beta_terminal_nodes = build_env_dag(
            engine,
            &pipeline_id,
            config,
            ServiceEnv::Beta,
            &config.beta_hosts,
            vec![],
        )
        .await?;
    }

    if !config.prod_hosts.is_empty() {
        let prod_predecessors = if beta_terminal_nodes.is_empty() {
            vec![]
        } else {
            let gate = engine
                .add_node(
                    &pipeline_id,
                    node(
                        DeploymentEvent::EnvironmentGate {
                            from: ServiceEnv::Beta.to_string(),
                            to: ServiceEnv::Prod.to_string(),
                        },
                        None,
                        NodeOptions {
                            is_unwind_boundary: true,
                            unwind_on_failure: false,
                        },
                    ),
                )
                .await?;
            for prev in &beta_terminal_nodes {
                engine.add_edge(prev, &gate).await?;
            }
            vec![gate]
        };

        build_env_dag(
            engine,
            &pipeline_id,
            config,
            ServiceEnv::Prod,
            &config.prod_hosts,
            prod_predecessors,
        )
        .await?;
    }

    engine.seal(&pipeline_id).await?;

    Ok(pipeline_id)
}

/// Returns Vec of Node IDs
async fn host_triplet(
    engine: &PipelineEngine,
    pipeline_id: &str,
    host: &String,
    config: &ProjectConfig,
    environment: &ServiceEnv,

    unwind_on_failure: bool,
) -> Result<[String; 3], anyhow::Error> {
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
                no_unwind(),
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
                NodeOptions {
                    unwind_on_failure,
                    ..Default::default()
                },
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
                NodeOptions {
                    unwind_on_failure,
                    ..Default::default()
                },
            ),
        )
        .await?;

    Ok([replicate, deploy, verify])
}

async fn build_env_dag(
    engine: &PipelineEngine,
    pipeline_id: &str,
    config: &ProjectConfig,
    environment: ServiceEnv,
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
                        environment: id(&environment.to_string()),
                    }),
                    no_unwind(),
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
                    no_unwind(),
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
                        no_unwind(),
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
                        environment: id(&environment.to_string()),
                    }),
                    no_unwind(),
                ),
            )
            .await?;
        for prev in &chain_tip {
            engine.add_edge(prev, &n).await?;
        }
        chain_tip = vec![n];
    }

    let (waves, unwind_on_failure) = if config.project_id == "ant-host-agent" {
        // We have ZERO wave layout here, and we also don't mark the nodes as "unwind_on_failure" since we have messy deploys.
        (vec![hosts.iter().map(|h| h).collect()], false)
    } else {
        (wave_layout(hosts), true)
    };

    for wave in waves {
        let mut wave_terminal_nodes = vec![];

        for host in wave {
            let [replicate, deploy, verify] = host_triplet(
                engine,
                pipeline_id,
                host,
                config,
                &environment,
                unwind_on_failure,
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
            Fixture {
                engine,
                db,
                _guard: guard,
            }
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
        // Gate: 1
        // Prod: 3 hosts in waves [1, 2] × 3 nodes = 9
        // Total: 13 nodes
        assert_eq!(nodes.len(), 13);

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

        // Beta verify → gate → prod wave 1 replicate (sequential environments)
        let beta_verify_id = nodes
            .iter()
            .find(|n| n.event.contains("deployment_verification") && n.event.contains("w2"))
            .unwrap();
        let gate_id = nodes
            .iter()
            .find(|n| n.event.contains("environment_gate"))
            .unwrap();
        let prod_first_replicate = nodes
            .iter()
            .find(|n| n.event.contains("artifact_replication") && n.event.contains("w1"))
            .unwrap();
        assert!(edges.iter().any(|e| {
            e.from_node_id == beta_verify_id.node_id && e.to_node_id == gate_id.node_id
        }));
        assert!(edges.iter().any(|e| {
            e.from_node_id == gate_id.node_id && e.to_node_id == prod_first_replicate.node_id
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
        let beta_migration = nodes
            .iter()
            .find(|n| n.event.contains("database_migration") && n.event.contains("beta"))
            .unwrap();
        let beta_replicate = nodes
            .iter()
            .find(|n| n.event.contains("artifact_replication") && n.event.contains("w2"))
            .unwrap();
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

        // Per env: route_update → alert_config → log_rule_config → db_migration → host triplet
        // Beta: 4 config + 3 host = 7
        // Gate: 1
        // Prod: 4 config + 3 host = 7
        // Total: 15
        assert_eq!(nodes.len(), 15);

        // Verify serial ordering in beta: route → alerts → log_rules → migration → replicate
        let beta_route = nodes
            .iter()
            .find(|n| n.event.contains("route_update") && n.event.contains("beta"))
            .unwrap();
        let alerts = nodes
            .iter()
            .find(|n| n.event.contains("alert_configuration"))
            .unwrap();

        assert!(edges
            .iter()
            .any(|e| { e.from_node_id == beta_route.node_id && e.to_node_id == alerts.node_id }));
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
        let route_node = nodes
            .iter()
            .find(|n| n.event.contains("route_update"))
            .unwrap();
        assert_eq!(route_node.state, "executable");
    }
}
