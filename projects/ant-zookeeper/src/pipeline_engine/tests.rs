use ant_library::db::TypesOfAntsDatabase;
use ant_library_test::db::TestDatabase;
use ant_zookeeper_db::AntZooStorageClient;
use tracing_test::traced_test;

use super::engine::{BlockReason, Edge, Job, Node, Pipeline, PipelineEngine};
use super::node::{NodeOptions, NodeSpec};

use crate::pipeline::resource_key::{DeploymentResource, Identifier};

fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}

mod nodes {
    use super::*;

    pub fn host_replicated(host_id: &str, service_id: &str) -> NodeSpec {
        let resource = DeploymentResource::HostService {
            host_id: id(host_id),
            service_id: id(service_id),
        };
        NodeSpec {
            event: serde_json::json!({
                "type": "host-replicated",
                "host_id": host_id,
                "service_id": service_id,
            })
            .to_string(),
            mutates: Some(resource.to_string()),
            options: NodeOptions::default(),
        }
    }

    pub fn host_deployed(host_id: &str, service_id: &str) -> NodeSpec {
        let resource = DeploymentResource::HostService {
            host_id: id(host_id),
            service_id: id(service_id),
        };
        NodeSpec {
            event: serde_json::json!({
                "type": "host-deployed",
                "host_id": host_id,
                "service_id": service_id,
            })
            .to_string(),
            mutates: Some(resource.to_string()),
            options: NodeOptions::default(),
        }
    }

    pub fn synthetic(name: &str) -> NodeSpec {
        NodeSpec {
            event: serde_json::json!({ "type": name }).to_string(),
            mutates: None,
            options: NodeOptions::default(),
        }
    }
}

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

        db.register_project("web", true).await.unwrap();
        db.register_project("gateway", true).await.unwrap();

        Fixture {
            engine,
            db,
            _guard: guard,
        }
    }

    async fn tick_all_done(&self) -> Vec<super::engine::Dispatch> {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let collected = Arc::new(Mutex::new(vec![]));
        let collected_clone = collected.clone();
        self.engine
            .tick(move |d| {
                let collected = collected_clone.clone();
                async move {
                    collected.lock().await.push(d);
                    Ok(())
                }
            })
            .await
            .unwrap()
            .join()
            .await
            .unwrap();

        Arc::try_unwrap(collected).unwrap().into_inner()
    }

    fn assert_unwind_dispatch(
        dispatch: &super::engine::Dispatch,
        expected_event_substr: &str,
        expected_restore_revision: Option<&str>,
    ) {
        assert!(
            dispatch.node.event.contains(expected_event_substr),
            "expected event containing '{}', got '{}'",
            expected_event_substr,
            dispatch.node.event
        );
        match &dispatch.direction {
            super::engine::DispatchDirection::Unwind {
                restore_revision_id,
            } => {
                assert_eq!(
                    restore_revision_id.as_deref(),
                    expected_restore_revision,
                    "wrong restore_revision_id for node '{}'",
                    expected_event_substr
                );
            }
            super::engine::DispatchDirection::Deploy => {
                panic!(
                    "expected Unwind dispatch for '{}', got Deploy",
                    expected_event_substr
                );
            }
        }
    }

    async fn tick_all_fail(&self) {
        self.engine
            .tick(|_| async { Err(anyhow::anyhow!("test failure")) })
            .await
            .unwrap()
            .join()
            .await
            .unwrap();
    }

    async fn assert_states(&self, pipeline_id: &str, expected: &[(&str, &str)]) {
        let nodes = self.engine.nodes(pipeline_id).await.unwrap();
        assert_eq!(
            nodes.len(),
            expected.len(),
            "Node count mismatch: got {} nodes but expected {}",
            nodes.len(),
            expected.len()
        );
        for (node, (expected_event_substr, expected_state)) in nodes.iter().zip(expected.iter()) {
            assert!(
                node.event.contains(expected_event_substr),
                "Expected event containing '{}', got '{}'",
                expected_event_substr,
                node.event
            );
            assert_eq!(
                node.state, *expected_state,
                "Node '{}' expected state '{}', got '{}'",
                expected_event_substr, expected_state, node.state
            );
        }
    }
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_single_node_becomes_runnable() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    f.engine
        .add_node(&p, nodes::synthetic("start"))
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(&p, &[("start", "executable")]).await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_successor_promoted_after_tick() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let a = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    let b = f
        .engine
        .add_node(&p, nodes::host_deployed("w1", "web"))
        .await
        .unwrap();
    f.engine.add_edge(&a, &b).await.unwrap();

    // Before seal: both pending
    f.assert_states(
        &p,
        &[("host-replicated", "pending"), ("host-deployed", "pending")],
    )
    .await;

    f.engine.seal(&p).await.unwrap();

    // After seal: root becomes executable, successor stays pending
    f.assert_states(
        &p,
        &[
            ("host-replicated", "executable"),
            ("host-deployed", "pending"),
        ],
    )
    .await;

    f.tick_all_done().await;

    f.assert_states(
        &p,
        &[
            ("host-replicated", "finished"),
            ("host-deployed", "executable"),
        ],
    )
    .await;

    f.tick_all_done().await;

    f.assert_states(
        &p,
        &[
            ("host-replicated", "finished"),
            ("host-deployed", "finished"),
        ],
    )
    .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_fifo_blocks_newer_revision() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(&p1, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(&p2, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();

    // Before seal: both pending
    f.assert_states(&p1, &[("host-replicated", "pending")])
        .await;
    f.assert_states(&p2, &[("host-replicated", "pending")])
        .await;

    f.engine.seal(&p1).await.unwrap();
    f.engine.seal(&p2).await.unwrap();

    // After seal: both "executable" in DB state, but FIFO means only rev1 is truly runnable
    f.assert_states(&p1, &[("host-replicated", "executable")])
        .await;
    f.assert_states(&p2, &[("host-replicated", "executable")])
        .await;
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 1);
    assert!(runnable[0].event.contains("w1"));

    f.tick_all_done().await;

    f.assert_states(&p1, &[("host-replicated", "finished")])
        .await;
    // Now rev2 is unblocked
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 1);

    f.tick_all_done().await;

    f.assert_states(&p2, &[("host-replicated", "finished")])
        .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_independent_resources_run_in_parallel() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("gateway").await.unwrap();

    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(&p1, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();

    let p2 = f.engine.create_pipeline("gateway", &rev2).await.unwrap();
    f.engine
        .add_node(&p2, nodes::host_replicated("w2", "gateway"))
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Different resources — both are truly runnable
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 2);

    f.tick_all_done().await;

    f.assert_states(&p1, &[("host-replicated", "finished")])
        .await;
    f.assert_states(&p2, &[("host-replicated", "finished")])
        .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_cancel_unblocks_newer_revision() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(&p1, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(&p2, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    f.engine.cancel(&p1).await.unwrap();

    f.assert_states(&p1, &[("host-replicated", "cancelled")])
        .await;
    f.assert_states(&p2, &[("host-replicated", "executable")])
        .await;

    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 1);
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_fan_out() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let root = f
        .engine
        .add_node(&p, nodes::synthetic("start"))
        .await
        .unwrap();
    let a = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    let b = f
        .engine
        .add_node(&p, nodes::host_replicated("w2", "web"))
        .await
        .unwrap();
    f.engine.add_edge(&root, &a).await.unwrap();
    f.engine.add_edge(&root, &b).await.unwrap();

    // Before seal: all pending
    f.assert_states(
        &p,
        &[("start", "pending"), ("w1", "pending"), ("w2", "pending")],
    )
    .await;

    f.engine.seal(&p).await.unwrap();

    // After seal: only root is executable (has no predecessors)
    f.assert_states(
        &p,
        &[
            ("start", "executable"),
            ("w1", "pending"),
            ("w2", "pending"),
        ],
    )
    .await;

    f.tick_all_done().await;

    f.assert_states(
        &p,
        &[
            ("start", "finished"),
            ("w1", "executable"),
            ("w2", "executable"),
        ],
    )
    .await;

    f.tick_all_done().await;

    f.assert_states(
        &p,
        &[
            ("start", "finished"),
            ("w1", "finished"),
            ("w2", "finished"),
        ],
    )
    .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_fan_in_waits_for_all_predecessors() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    // DAG: a → b → join ← c
    // "join" has two predecessors: b and c. c is a root, b requires a first.
    // This means c finishes before b (since b has to wait for a).
    // join should NOT be promoted when c finishes — it must wait for b too.
    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let a = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    let b = f
        .engine
        .add_node(&p, nodes::host_deployed("w1", "web"))
        .await
        .unwrap();
    let c = f
        .engine
        .add_node(&p, nodes::host_replicated("w2", "web"))
        .await
        .unwrap();
    let join = f
        .engine
        .add_node(&p, nodes::synthetic("all-done"))
        .await
        .unwrap();
    f.engine.add_edge(&a, &b).await.unwrap();
    f.engine.add_edge(&b, &join).await.unwrap();
    f.engine.add_edge(&c, &join).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    // After seal: a and c are executable (roots), b and join are pending
    f.assert_states(
        &p,
        &[
            ("w1\",\"service_id\":\"web", "executable"), // a (host-replicated w1)
            ("host-deployed", "pending"),                // b
            ("w2", "executable"),                        // c (host-replicated w2)
            ("all-done", "pending"),                     // join
        ],
    )
    .await;

    // Tick 1: a and c both complete
    f.tick_all_done().await;

    // b is promoted (a finished), but join is NOT (b still pending)
    f.assert_states(
        &p,
        &[
            ("w1\",\"service_id\":\"web", "finished"),
            ("host-deployed", "executable"),
            ("w2", "finished"),
            ("all-done", "pending"),
        ],
    )
    .await;

    // Tick 2: b completes
    f.tick_all_done().await;

    // NOW join is promoted (both predecessors b and c are finished)
    f.assert_states(
        &p,
        &[
            ("w1\",\"service_id\":\"web", "finished"),
            ("host-deployed", "finished"),
            ("w2", "finished"),
            ("all-done", "executable"),
        ],
    )
    .await;

    // Tick 3: join completes
    f.tick_all_done().await;

    f.assert_states(
        &p,
        &[
            ("w1\",\"service_id\":\"web", "finished"),
            ("host-deployed", "finished"),
            ("w2", "finished"),
            ("all-done", "finished"),
        ],
    )
    .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_failed_node_does_not_retry_automatically() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    f.engine
        .add_node(&p, NodeSpec {
            event: serde_json::json!({"type": "host-replicated", "host_id": "w1", "service_id": "web"}).to_string(),
            mutates: Some("host_service:w1:web".to_string()),
            options: NodeOptions { unwind_on_failure: false, ..NodeOptions::default() },
        })
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(&p, &[("host-replicated", "executable")])
        .await;

    // Fail
    f.tick_all_fail().await;

    // Node is failed, NOT executable
    f.assert_states(&p, &[("host-replicated", "failed")]).await;

    // Next tick does nothing — node stays failed, is not retried
    f.tick_all_done().await;

    f.assert_states(&p, &[("host-replicated", "failed")]).await;

    // runnable() returns nothing
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 0);
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_manual_retry_makes_node_executable() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let n = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    f.tick_all_fail().await;

    f.assert_states(&p, &[("host-replicated", "failed")]).await;

    // Manual retry
    f.engine.retry(&n).await.unwrap();

    f.assert_states(&p, &[("host-replicated", "executable")])
        .await;

    // Now tick succeeds
    f.tick_all_done().await;

    f.assert_states(&p, &[("host-replicated", "finished")])
        .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_stale_heartbeat_auto_retries() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let n = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(&p, &[("host-replicated", "executable")])
        .await;

    // Simulate a crashed process: node is in_progress with a stale heartbeat
    let pool = f.db.pool();
    let con = pool.get().await.unwrap();
    con.execute(
        "update pipeline_engine_node set state = 'in_progress', started_at = now() where node_id = $1",
        &[&n],
    ).await.unwrap();
    con.execute(
        "insert into pipeline_engine_node_event (node_id, to_state, reason) values ($1, 'in_progress', 'test_simulate_crash')",
        &[&n],
    ).await.unwrap();
    con.query_one(
        "insert into pipeline_engine_job (node_id, last_heartbeat_at) values ($1, now() - interval '120 seconds') returning job_id",
        &[&n],
    ).await.unwrap();

    f.assert_states(&p, &[("host-replicated", "in_progress")])
        .await;

    // Next tick detects stale heartbeat, releases node to executable, then re-dispatches
    f.tick_all_done().await;

    f.assert_states(&p, &[("host-replicated", "finished")])
        .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_why_blocked_consistent_with_runnable() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Pipeline 1: a → b (sequential)
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    let a = f
        .engine
        .add_node(&p1, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    let b = f
        .engine
        .add_node(&p1, nodes::host_deployed("w1", "web"))
        .await
        .unwrap();
    f.engine.add_edge(&a, &b).await.unwrap();
    f.engine.seal(&p1).await.unwrap();

    // Pipeline 2: same resource, blocked by FIFO
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    let c = f
        .engine
        .add_node(&p2, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    let runnable = f.engine.runnable().await.unwrap();
    let runnable_ids: Vec<&str> = runnable.iter().map(|n| n.node_id.as_str()).collect();

    // a is runnable: no predecessors, no FIFO contention
    assert!(runnable_ids.contains(&a.as_str()));
    assert!(f.engine.why_blocked(&a).await.unwrap().is_none());

    // b is not runnable: it's pending because predecessor a hasn't finished
    assert!(!runnable_ids.contains(&b.as_str()));
    let reason = f.engine.why_blocked(&b).await.unwrap().unwrap();
    match &reason {
        BlockReason::NotExecutable {
            state,
            pending_predecessors,
        } => {
            assert_eq!(state, "pending");
            assert_eq!(pending_predecessors.len(), 1);
            assert_eq!(pending_predecessors[0].node_id, a);
        }
        other => panic!("Expected NotExecutable, got: {other:?}"),
    }

    // c is not runnable: resource contention (rev1 has incomplete work on same resource)
    assert!(!runnable_ids.contains(&c.as_str()));
    let reason = f.engine.why_blocked(&c).await.unwrap().unwrap();
    assert!(matches!(reason, BlockReason::ResourceContention { .. }));
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_failed_node_blocks_newer_revision_until_cancel() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(&p1, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(&p2, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();

    // Before seal: both pending
    f.assert_states(&p1, &[("host-replicated", "pending")])
        .await;
    f.assert_states(&p2, &[("host-replicated", "pending")])
        .await;

    f.engine.seal(&p1).await.unwrap();
    f.engine.seal(&p2).await.unwrap();

    // After seal: both executable in DB, but FIFO blocks rev2
    f.assert_states(&p1, &[("host-replicated", "executable")])
        .await;
    f.assert_states(&p2, &[("host-replicated", "executable")])
        .await;

    // Fail rev1
    f.tick_all_fail().await;

    f.assert_states(&p1, &[("host-replicated", "failed")]).await;
    f.assert_states(&p2, &[("host-replicated", "executable")])
        .await;

    // rev2 is blocked by FIFO (rev1 failed, not finished/cancelled)
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 0);

    // Cancel rev1 to unblock rev2
    f.engine.cancel(&p1).await.unwrap();

    // cancel marks all non-finished nodes as cancelled (including failed)
    f.assert_states(&p1, &[("host-replicated", "cancelled")])
        .await;
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 1);

    f.tick_all_done().await;

    f.assert_states(&p2, &[("host-replicated", "finished")])
        .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_fifo_blocks_entire_chain_on_same_resource() {
    use std::sync::Arc;
    use tokio::sync::Notify;

    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    let resource = "host_service:w1:web";

    let p1 = {
        let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
        let start = f
            .engine
            .add_node(&p, nodes::synthetic("start"))
            .await
            .unwrap();
        let n1 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "replicate", "host": "w1"}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let n2 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "deploy", "host": "w1"}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let n3 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "verify", "host": "w1"}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let end = f
            .engine
            .add_node(&p, nodes::synthetic("end"))
            .await
            .unwrap();
        f.engine.add_edge(&start, &n1).await.unwrap();
        f.engine.add_edge(&n1, &n2).await.unwrap();
        f.engine.add_edge(&n2, &n3).await.unwrap();
        f.engine.add_edge(&n3, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        p
    };

    // [rev1, start] succeeds
    f.tick_all_done().await;

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("replicate", "executable"),
            ("deploy", "pending"),
            ("verify", "pending"),
            ("end", "pending"),
        ],
    )
    .await;

    // [rev1, node1] succeeds
    f.tick_all_done().await;

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("replicate", "finished"),
            ("deploy", "executable"),
            ("verify", "pending"),
            ("end", "pending"),
        ],
    )
    .await;

    // [rev1, node2] succeeds
    f.tick_all_done().await;

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("replicate", "finished"),
            ("deploy", "finished"),
            ("verify", "executable"),
            ("end", "pending"),
        ],
    )
    .await;

    // [rev2, start] — rev2 arrives mid-pipeline while rev1 is between node2 and node3
    let rev2 = f.db.create_revision("web").await.unwrap();
    let p2 = {
        let p = f.engine.create_pipeline("web", &rev2).await.unwrap();
        let start = f
            .engine
            .add_node(&p, nodes::synthetic("start"))
            .await
            .unwrap();
        let n1 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "replicate", "host": "w1"}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let n2 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "deploy", "host": "w1"}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let n3 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "verify", "host": "w1"}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let end = f
            .engine
            .add_node(&p, nodes::synthetic("end"))
            .await
            .unwrap();
        f.engine.add_edge(&start, &n1).await.unwrap();
        f.engine.add_edge(&n1, &n2).await.unwrap();
        f.engine.add_edge(&n2, &n3).await.unwrap();
        f.engine.add_edge(&n3, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        p
    };

    // Rev2 exists — start is runnable (no resource key), but node1 is FIFO-blocked
    f.assert_states(
        &p2,
        &[
            ("start", "executable"),
            ("replicate", "pending"),
            ("deploy", "pending"),
            ("verify", "pending"),
            ("end", "pending"),
        ],
    )
    .await;

    // Only rev1's verify and rev2's start are runnable
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 2);

    // [rev1, node3] starts as long-running, [rev2, start] also runs (no resource contention)
    let gate = Arc::new(Notify::new());

    let tick_handle = {
        let gate = gate.clone();
        f.engine
            .tick(move |d| {
                let node = d.node.clone();
                let gate = gate.clone();
                async move {
                    if node.event.contains("verify") {
                        gate.notified().await;
                    }
                    Ok(())
                }
            })
            .await
            .unwrap()
    };

    // [rev1, node3] completes → [rev1, end] promoted
    gate.notify_one();
    tick_handle.join().await.unwrap();

    // After join: rev1 verify finished, rev2 start finished (both dispatched in same tick)
    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("replicate", "finished"),
            ("deploy", "finished"),
            ("verify", "finished"),
            ("end", "executable"),
        ],
    )
    .await;

    f.assert_states(
        &p2,
        &[
            ("start", "finished"),
            ("replicate", "executable"),
            ("deploy", "pending"),
            ("verify", "pending"),
            ("end", "pending"),
        ],
    )
    .await;

    // [rev1, end] completes — rev1 fully done
    f.tick_all_done().await;

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("replicate", "finished"),
            ("deploy", "finished"),
            ("verify", "finished"),
            ("end", "finished"),
        ],
    )
    .await;

    // [rev2, node1] [rev2, node2] [rev2, node3] [rev2, end] — now unblocked
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.tick_all_done().await;

    f.assert_states(
        &p2,
        &[
            ("start", "finished"),
            ("replicate", "finished"),
            ("deploy", "finished"),
            ("verify", "finished"),
            ("end", "finished"),
        ],
    )
    .await;
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_fan_in_promotion_under_concurrent_succeed() {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let f = Fixture::new().await;

    for iteration in 0..20 {
        let rev = f.db.create_revision("web").await.unwrap();

        let p = f.engine.create_pipeline("web", &rev).await.unwrap();
        let mut predecessors = vec![];
        for i in 0..10 {
            let n = f
                .engine
                .add_node(
                    &p,
                    NodeSpec {
                        event:
                            serde_json::json!({"type": "predecessor", "i": i, "iter": iteration})
                                .to_string(),
                        mutates: None,
                        options: NodeOptions::default(),
                    },
                )
                .await
                .unwrap();
            predecessors.push(n);
        }
        let end = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "join", "iter": iteration}).to_string(),
                    mutates: None,
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        for pred in &predecessors {
            f.engine.add_edge(pred, &end).await.unwrap();
        }
        f.engine.seal(&p).await.unwrap();

        let barrier = Arc::new(Barrier::new(10));

        let tick_handle = {
            let barrier = barrier.clone();
            f.engine
                .tick(move |_| {
                    let barrier = barrier.clone();
                    async move {
                        barrier.wait().await;
                        Ok(())
                    }
                })
                .await
                .unwrap()
        };

        tick_handle.join().await.unwrap();

        let nodes = f.engine.nodes(&p).await.unwrap();
        let end_node = nodes.iter().find(|n| n.event.contains("join")).unwrap();

        // Between ticks, fan-in node may be pending (concurrent succeed() race)
        // or executable (last succeed() saw all commits). Both are valid.
        assert!(
            end_node.state == "pending" || end_node.state == "executable",
            "iteration {iteration}: fan-in node in unexpected state '{}' after all predecessors finished",
            end_node.state,
        );

        // Next tick's promote_unblocked_nodes() guarantees it becomes executable + finishes
        f.tick_all_done().await;

        let nodes = f.engine.nodes(&p).await.unwrap();
        let end_node = nodes.iter().find(|n| n.event.contains("join")).unwrap();
        assert_eq!(
            end_node.state, "finished",
            "iteration {iteration}: fan-in node not finished after tick"
        );
    }
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_stress_concurrent_ticks_no_double_dispatch() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let f = Fixture::new().await;

    let mut revisions = vec![];
    for _ in 0..20 {
        revisions.push(f.db.create_revision("web").await.unwrap());
    }

    let resource = "host_service:w1:web";
    let mut pipeline_ids = vec![];

    for rev in &revisions {
        let p = f.engine.create_pipeline("web", rev).await.unwrap();
        f.engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "work", "rev": rev}).to_string(),
                    mutates: Some(resource.to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        f.engine.seal(&p).await.unwrap();
        pipeline_ids.push(p);
    }

    let dispatch_count = Arc::new(AtomicU32::new(0));
    let max_concurrent = Arc::new(AtomicU32::new(0));
    let concurrent_count = Arc::new(AtomicU32::new(0));

    for _ in 0..200 {
        let barrier = Arc::new(Barrier::new(20));
        let mut handles = vec![];

        for _ in 0..20 {
            let pool = f.db.pool();
            let barrier = barrier.clone();
            let dispatch_count = dispatch_count.clone();
            let concurrent_count = concurrent_count.clone();
            let max_concurrent = max_concurrent.clone();

            handles.push(tokio::spawn(async move {
                barrier.wait().await;

                let engine = PipelineEngine::new(pool).await.unwrap();
                let dc = dispatch_count.clone();
                let cc = concurrent_count.clone();
                let mc = max_concurrent.clone();

                let tick_handle = engine
                    .tick(move |_node| {
                        let dc = dc.clone();
                        let cc = cc.clone();
                        let mc = mc.clone();
                        async move {
                            dc.fetch_add(1, Ordering::SeqCst);
                            let current = cc.fetch_add(1, Ordering::SeqCst) + 1;
                            mc.fetch_max(current, Ordering::SeqCst);
                            tokio::task::yield_now().await;
                            cc.fetch_sub(1, Ordering::SeqCst);
                            Ok(())
                        }
                    })
                    .await
                    .unwrap();

                tick_handle.join().await.unwrap();
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let active = f.engine.active_pipelines("web").await.unwrap();
        if active.is_empty() {
            break;
        }
    }

    // Invariant 1: exactly 20 dispatches (one per revision, never double-dispatched)
    assert_eq!(dispatch_count.load(Ordering::SeqCst), 20);

    // Invariant 2: max concurrency on same resource == 1 (FIFO ensures serial)
    assert_eq!(max_concurrent.load(Ordering::SeqCst), 1);

    // Invariant 3: all pipelines finished
    for pipeline_id in &pipeline_ids {
        let nodes = f.engine.nodes(pipeline_id).await.unwrap();
        assert_eq!(nodes[0].state, "finished");
    }
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_edges_returns_dag_structure() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let a = f
        .engine
        .add_node(&p, nodes::synthetic("start"))
        .await
        .unwrap();
    let b = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    let c = f
        .engine
        .add_node(&p, nodes::host_replicated("w2", "web"))
        .await
        .unwrap();
    let d = f
        .engine
        .add_node(&p, nodes::synthetic("end"))
        .await
        .unwrap();
    f.engine.add_edge(&a, &b).await.unwrap();
    f.engine.add_edge(&a, &c).await.unwrap();
    f.engine.add_edge(&b, &d).await.unwrap();
    f.engine.add_edge(&c, &d).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    let edges = f.engine.edges(&p).await.unwrap();
    assert_eq!(edges.len(), 4);

    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| (e.from_node_id.as_str(), e.to_node_id.as_str()))
        .collect();
    assert!(pairs.contains(&(a.as_str(), b.as_str())));
    assert!(pairs.contains(&(a.as_str(), c.as_str())));
    assert!(pairs.contains(&(b.as_str(), d.as_str())));
    assert!(pairs.contains(&(c.as_str(), d.as_str())));
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_nodes_layered_returns_topological_columns() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("ant-on-the-web").await.unwrap();

    // DAG: start → [m1, m2] → end
    let p = f
        .engine
        .create_pipeline("ant-on-the-web", &rev)
        .await
        .unwrap();
    let start = f
        .engine
        .add_node(&p, nodes::synthetic("start"))
        .await
        .unwrap();
    let m1 = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    let m2 = f
        .engine
        .add_node(&p, nodes::host_replicated("w2", "web"))
        .await
        .unwrap();
    let end = f
        .engine
        .add_node(&p, nodes::synthetic("end"))
        .await
        .unwrap();
    f.engine.add_edge(&start, &m1).await.unwrap();
    f.engine.add_edge(&start, &m2).await.unwrap();
    f.engine.add_edge(&m1, &end).await.unwrap();
    f.engine.add_edge(&m2, &end).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    let layers = f.engine.nodes_layered(&p).await.unwrap();

    assert_eq!(layers.len(), 3);

    // Layer 0: start (root, no predecessors)
    assert_eq!(layers[0].len(), 1);
    assert!(layers[0][0].event.contains("start"));

    // Layer 1: m1, m2 (parallel, both depend only on start)
    assert_eq!(layers[1].len(), 2);
    let layer1_events: Vec<&str> = layers[1].iter().map(|n| n.event.as_str()).collect();
    assert!(layer1_events.iter().any(|e| e.contains("w1")));
    assert!(layer1_events.iter().any(|e| e.contains("w2")));

    // Layer 2: end (depends on both m1 and m2)
    assert_eq!(layers[2].len(), 1);
    assert!(layers[2][0].event.contains("end"));
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_nodes_layered_sequential_chain() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("ant-on-the-web").await.unwrap();

    // DAG: a → b → c → d
    let p = f
        .engine
        .create_pipeline("ant-on-the-web", &rev)
        .await
        .unwrap();
    let a = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "a"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let b = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "b"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let c = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "c"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let d = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "d"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&a, &b).await.unwrap();
    f.engine.add_edge(&b, &c).await.unwrap();
    f.engine.add_edge(&c, &d).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    let layers = f.engine.nodes_layered(&p).await.unwrap();

    assert_eq!(layers.len(), 4);
    assert_eq!(layers[0].len(), 1);
    assert!(layers[0][0].event.contains("\"a\""));
    assert_eq!(layers[1].len(), 1);
    assert!(layers[1][0].event.contains("\"b\""));
    assert_eq!(layers[2].len(), 1);
    assert!(layers[2][0].event.contains("\"c\""));
    assert_eq!(layers[3].len(), 1);
    assert!(layers[3][0].event.contains("\"d\""));
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_active_pipelines_returns_in_progress() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(&p1, nodes::synthetic("start"))
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(&p2, nodes::synthetic("start"))
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 2);

    f.tick_all_done().await;

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 0);
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_latest_finished_pipeline() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Same resource key — FIFO ensures p1 finishes before p2 can start
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(&p1, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(&p2, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Nothing finished yet
    let latest = f.engine.latest_finished_pipeline("web").await.unwrap();
    assert!(latest.is_none());

    // Tick 1: only p1 runs (FIFO blocks p2)
    f.tick_all_done().await;

    // p1 is finished, latest should be p1
    let latest = f.engine.latest_finished_pipeline("web").await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().pipeline_id, p1);

    // Tick 2: p2 now unblocked and runs
    f.tick_all_done().await;

    // p2 is finished, latest should be p2 (higher revision_seq)
    let latest = f.engine.latest_finished_pipeline("web").await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().pipeline_id, p2);
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_node_job_returns_latest_job() {
    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let n = f
        .engine
        .add_node(&p, nodes::host_replicated("w1", "web"))
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    let job = f.engine.node_job(&n).await.unwrap();
    assert!(job.is_none());

    f.tick_all_fail().await;

    let job = f.engine.node_job(&n).await.unwrap().unwrap();
    assert_eq!(job.node_id, n);
    assert_eq!(job.state, "failed");

    f.engine.retry(&n).await.unwrap();
    f.tick_all_done().await;

    let job = f.engine.node_job(&n).await.unwrap().unwrap();
    assert_eq!(job.state, "succeeded");
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_e2e_concurrent_revisions_with_long_running_jobs() {
    use std::sync::Arc;
    use tokio::sync::Notify;

    let f = Fixture::new().await;

    let rev1 = f.db.create_revision("web").await.unwrap();
    let (p1, m1, m2) = {
        let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
        let start = f
            .engine
            .add_node(&p, nodes::synthetic("start"))
            .await
            .unwrap();
        let m1 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "middle", "name": "m1"}).to_string(),
                    mutates: Some("m1-resource".to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let m2 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "middle", "name": "m2"}).to_string(),
                    mutates: Some("m2-resource".to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let end = f
            .engine
            .add_node(&p, nodes::synthetic("end"))
            .await
            .unwrap();
        f.engine.add_edge(&start, &m1).await.unwrap();
        f.engine.add_edge(&start, &m2).await.unwrap();
        f.engine.add_edge(&m1, &end).await.unwrap();
        f.engine.add_edge(&m2, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        (p, m1, m2)
    };

    f.tick_all_done().await;

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("m1", "executable"),
            ("m2", "executable"),
            ("end", "pending"),
        ],
    )
    .await;

    let m1_gate = Arc::new(Notify::new());
    let m2_gate = Arc::new(Notify::new());

    let tick_handle = {
        let m1_gate = m1_gate.clone();
        let m2_gate = m2_gate.clone();
        f.engine
            .tick(move |d| {
                let node = d.node.clone();
                let m1_g = m1_gate.clone();
                let m2_g = m2_gate.clone();
                async move {
                    if node.event.contains("m1") {
                        m1_g.notified().await;
                    } else if node.event.contains("m2") {
                        m2_g.notified().await;
                    }
                    Ok(())
                }
            })
            .await
            .unwrap()
    };

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("m1", "in_progress"),
            ("m2", "in_progress"),
            ("end", "pending"),
        ],
    )
    .await;

    assert_eq!(
        f.engine.node_job(&m1).await.unwrap().unwrap().state,
        "running"
    );
    assert_eq!(
        f.engine.node_job(&m2).await.unwrap().unwrap().state,
        "running"
    );

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].pipeline_id, p1);

    let rev2 = f.db.create_revision("web").await.unwrap();
    let p2 = {
        let p = f.engine.create_pipeline("web", &rev2).await.unwrap();
        let start = f
            .engine
            .add_node(&p, nodes::synthetic("start"))
            .await
            .unwrap();
        let m1 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "middle", "name": "m1"}).to_string(),
                    mutates: Some("m1-resource".to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let m2 = f
            .engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "middle", "name": "m2"}).to_string(),
                    mutates: Some("m2-resource".to_string()),
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        let end = f
            .engine
            .add_node(&p, nodes::synthetic("end"))
            .await
            .unwrap();
        f.engine.add_edge(&start, &m1).await.unwrap();
        f.engine.add_edge(&start, &m2).await.unwrap();
        f.engine.add_edge(&m1, &end).await.unwrap();
        f.engine.add_edge(&m2, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        p
    };

    f.tick_all_done().await;

    f.assert_states(
        &p2,
        &[
            ("start", "finished"),
            ("m1", "executable"),
            ("m2", "executable"),
            ("end", "pending"),
        ],
    )
    .await;

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 2);

    drop(tick_handle);

    {
        let pool = f.db.pool();
        let con = pool.get().await.unwrap();
        con.execute(
            "update pipeline_engine_node set state = 'failed', finished_at = now(), updated_at = now() where node_id = any($1)",
            &[&vec![&m1, &m2]],
        ).await.unwrap();
        con.execute(
            "update pipeline_engine_job set state = 'failed', error = 'test crash', finished_at = now(), updated_at = now() where node_id = any($1)",
            &[&vec![&m1, &m2]],
        ).await.unwrap();
    }

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("m1", "failed"),
            ("m2", "failed"),
            ("end", "pending"),
        ],
    )
    .await;

    f.engine.cancel(&p1).await.unwrap();

    f.assert_states(
        &p1,
        &[
            ("start", "finished"),
            ("m1", "cancelled"),
            ("m2", "cancelled"),
            ("end", "cancelled"),
        ],
    )
    .await;

    let m1_gate2 = Arc::new(Notify::new());
    let m2_gate2 = Arc::new(Notify::new());

    let tick_handle2 = {
        let m1_gate = m1_gate2.clone();
        let m2_gate = m2_gate2.clone();
        f.engine
            .tick(move |d| {
                let node = d.node.clone();
                let m1_g = m1_gate.clone();
                let m2_g = m2_gate.clone();
                async move {
                    if node.event.contains("m1") {
                        m1_g.notified().await;
                    } else if node.event.contains("m2") {
                        m2_g.notified().await;
                    }
                    Ok(())
                }
            })
            .await
            .unwrap()
    };

    f.assert_states(
        &p2,
        &[
            ("start", "finished"),
            ("m1", "in_progress"),
            ("m2", "in_progress"),
            ("end", "pending"),
        ],
    )
    .await;

    m1_gate2.notify_one();
    m2_gate2.notify_one();
    tick_handle2.join().await.unwrap();

    // Next tick promotes end (may be pending due to concurrent succeed() race) and executes it
    f.tick_all_done().await;

    f.assert_states(
        &p2,
        &[
            ("start", "finished"),
            ("m1", "finished"),
            ("m2", "finished"),
            ("end", "finished"),
        ],
    )
    .await;

    let latest = f
        .engine
        .latest_finished_pipeline("web")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.pipeline_id, p2);

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 0);
}

// [m1(r1) -> m2(r2) -> m3(r3)], single revision, no prior state.
// Rev1 deploys m1 and m2 successfully, then fails on m3. The engine detects the failure
// on the next tick and automatically begins unwinding in reverse order: m3 first (failed
// node, may have partial state), then m2, then m1.
#[tokio::test]
#[traced_test]
async fn unwind_linear_no_prior_revision() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    // [m1(r1) -> m2(r2) -> m3(r3)]
    let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
    let m1 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m2 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m2"}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m3 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m3"}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&m1, &m2).await.unwrap();
    f.engine.add_edge(&m2, &m3).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(
        &p,
        &[("m1", "executable"), ("m2", "pending"), ("m3", "pending")],
    )
    .await;

    // Forward tick 1: m1 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;

    // Forward tick 2: m2 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;

    // Forward tick 3: m3 fails — because unwind_on_failure=true, engine automatically
    // transitions pipeline to 'unwinding' and dispatches m3 for unwind in the same tick
    f.tick_all_fail().await;
    f.assert_states(
        &p,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "failed")],
    )
    .await;

    // Tick 4: engine detects failed node with unwind_on_failure, pipeline is now unwinding,
    // dispatches m3 for unwind (leaf in scope, no successors to wait for)
    let dispatches = f.tick_all_done().await;
    assert_eq!(dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(&dispatches[0], "m3", None);
    f.assert_states(
        &p,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "unwound")],
    )
    .await;

    // Tick 5: m2 is now unwind-eligible (successor m3 is unwound)
    let dispatches = f.tick_all_done().await;
    assert_eq!(dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(&dispatches[0], "m2", None);
    f.assert_states(
        &p,
        &[("m1", "finished"), ("m2", "unwound"), ("m3", "unwound")],
    )
    .await;

    // Tick 6: m1 is now unwind-eligible (successor m2 is unwound)
    let dispatches = f.tick_all_done().await;
    assert_eq!(dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(&dispatches[0], "m1", None);
    f.assert_states(
        &p,
        &[("m1", "unwound"), ("m2", "unwound"), ("m3", "unwound")],
    )
    .await;

    // Next tick: pipeline completes — all nodes terminal, transitions to 'unwound'
    f.tick_all_done().await;
    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(
        active.len(),
        0,
        "pipeline should no longer be active after unwind completes"
    );

    // An unwound pipeline is NOT a "finished" pipeline — latest_finished should be None
    let latest = f.engine.latest_finished_pipeline("web").await.unwrap();
    assert!(
        latest.is_none(),
        "unwound pipeline should not appear as latest finished"
    );
}

// [m1(r1) -> m2(r2) -> m3(r3)], two revisions.
// Rev1 fully deploys. Rev2 deploys m1+m2, fails on m3. After rev2's unwind completes,
// rev1's nodes remain finished — effective resource state is rev1 everywhere.
#[tokio::test]
#[traced_test]
async fn unwind_linear_restores_to_prior_revision() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2) -> m3(r3)]
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    let p1_m1 = f
        .engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_m2 = f
        .engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_m3 = f
        .engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 1}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&p1_m1, &p1_m2).await.unwrap();
    f.engine.add_edge(&p1_m2, &p1_m3).await.unwrap();
    f.engine.seal(&p1).await.unwrap();

    // Rev1 fully finishes
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;

    // Rev2: [m1(r1) -> m2(r2) -> m3(r3)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 2}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    f.engine
        .add_edge(&p2_nodes[0].node_id, &p2_nodes[1].node_id)
        .await
        .unwrap();
    f.engine
        .add_edge(&p2_nodes[1].node_id, &p2_nodes[2].node_id)
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1 succeeds, m2 succeeds, m3 fails (triggers auto-unwind)
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;
    f.tick_all_fail().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "failed")],
    )
    .await;

    // Unwind: m3, then m2, then m1 — each should restore to rev1
    let dispatches = f.tick_all_done().await;
    assert_eq!(dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(&dispatches[0], "m3", Some(&rev1));
    assert_eq!(dispatches[0].revision_id, rev2);
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "unwound")],
    )
    .await;

    let dispatches = f.tick_all_done().await;
    assert_eq!(dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(&dispatches[0], "m2", Some(&rev1));
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "unwound"), ("m3", "unwound")],
    )
    .await;

    let dispatches = f.tick_all_done().await;
    assert_eq!(dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(&dispatches[0], "m1", Some(&rev1));
    f.assert_states(
        &p2,
        &[("m1", "unwound"), ("m2", "unwound"), ("m3", "unwound")],
    )
    .await;

    // Rev1's nodes untouched — resources are effectively at rev1
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;

    // Pipeline completion: rev2 is unwound, rev1 is still the latest finished
    f.tick_all_done().await;
    let latest = f.engine.latest_finished_pipeline("web").await.unwrap();
    assert_eq!(
        latest.unwrap().pipeline_id,
        p1,
        "latest finished should be rev1, not the unwound rev2"
    );
    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 0);
}

// [m1(r1) -> m2(r2) -> gate -> m3(r3) -> m4(r4)] where gate has is_unwind_boundary=true.
// Rev1 finishes fully. Rev2 deploys everything, fails on m4. Unwind walks back from m4,
// hits gate, stops. Only m3 and m4 are in unwind scope; m1, m2, gate stay finished.
#[tokio::test]
#[traced_test]
async fn unwind_stops_at_gate_node() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2) -> gate -> m3(r3) -> m4(r4)]
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "gate", "rev": 1}).to_string(),
                mutates: None,
                options: NodeOptions {
                    is_unwind_boundary: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 1}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m4", "rev": 1}).to_string(),
                mutates: Some("r4".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    for i in 0..4 {
        f.engine
            .add_edge(&p1_nodes[i].node_id, &p1_nodes[i + 1].node_id)
            .await
            .unwrap();
    }
    f.engine.seal(&p1).await.unwrap();

    // Rev1: finish all 5 nodes
    for _ in 0..5 {
        f.tick_all_done().await;
    }
    f.assert_states(
        &p1,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "finished"),
            ("m4", "finished"),
        ],
    )
    .await;

    // Rev2: [m1(r1) -> m2(r2) -> gate -> m3(r3) -> m4(r4)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "gate", "rev": 2}).to_string(),
                mutates: None,
                options: NodeOptions {
                    is_unwind_boundary: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 2}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m4", "rev": 2}).to_string(),
                mutates: Some("r4".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    for i in 0..4 {
        f.engine
            .add_edge(&p2_nodes[i].node_id, &p2_nodes[i + 1].node_id)
            .await
            .unwrap();
    }
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: linear chain, one node dispatched per tick
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "executable"),
            ("gate", "pending"),
            ("m3", "pending"),
            ("m4", "pending"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "executable"),
            ("m3", "pending"),
            ("m4", "pending"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "executable"),
            ("m4", "pending"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "finished"),
            ("m4", "executable"),
        ],
    )
    .await;
    f.tick_all_fail().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "finished"),
            ("m4", "failed"),
        ],
    )
    .await;

    // Unwind tick 1: auto-unwind triggered (m4 has unwind_on_failure),
    // scope walks back from m4, stops at gate. m4 dispatched (leaf in scope).
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "finished"),
            ("m4", "unwound"),
        ],
    )
    .await;

    // Unwind tick 2: m3 (successor m4 is unwound)
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "unwound"),
            ("m4", "unwound"),
        ],
    )
    .await;

    // m1, m2, gate remain finished — rev2 is still deployed pre-gate
    // Rev1's nodes untouched
    f.assert_states(
        &p1,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("gate", "finished"),
            ("m3", "finished"),
            ("m4", "finished"),
        ],
    )
    .await;
}

// [m1(r1) -> [m2(r2), m3(r3)] -> m4(r4, unwind_on_failure)] (fan-out then fan-in).
// m1, m2, m3 succeed, then m4 fails. Unwind scope is {m4, m2, m3, m1}.
// m4 unwinds first, then m2 and m3 in parallel. m2's unwind succeeds but m3's fails on
// first attempt — verifies that m1 is blocked until m3 is retried and succeeds.
#[tokio::test]
#[traced_test]
async fn unwind_fan_out_blocks_until_all_branches_unwound_including_retries() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    // [m1(r1) -> [m2(r2), m3(r3)] -> m4(r4, unwind_on_failure)]
    let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
    let m1 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m2 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m2"}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m3 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m3"}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m4 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m4"}).to_string(),
                mutates: Some("r4".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&m1, &m2).await.unwrap();
    f.engine.add_edge(&m1, &m3).await.unwrap();
    f.engine.add_edge(&m2, &m4).await.unwrap();
    f.engine.add_edge(&m3, &m4).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    // Forward: m1 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "executable"),
            ("m3", "executable"),
            ("m4", "pending"),
        ],
    )
    .await;

    // Forward: m2 and m3 succeed in parallel. m4 may be promoted inline by succeed()
    // (if one commits before the other's promote query) or stay pending (race).
    f.tick_all_done().await;
    let nodes = f.engine.nodes(&p).await.unwrap();
    let m4_node = nodes.iter().find(|n| n.event.contains("m4")).unwrap();
    assert!(
        m4_node.state == "pending" || m4_node.state == "executable",
        "m4 should be pending or executable after concurrent succeed, got '{}'",
        m4_node.state
    );

    // Forward: m4 fails (promote_unblocked_nodes ensures m4 is executable, then it fails)
    f.tick_all_fail().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "finished"),
            ("m4", "failed"),
        ],
    )
    .await;

    // Unwind tick 1: m4 is leaf in scope, dispatched for unwind
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "finished"),
            ("m4", "unwound"),
        ],
    )
    .await;

    // Unwind tick 2: m2 and m3 are both eligible (successor m4 is unwound).
    // m2 succeeds, m3's unwind handler fails.
    let m3_should_fail = Arc::new(AtomicBool::new(true));
    let flag = m3_should_fail.clone();
    f.engine
        .tick(move |d| {
            let node = d.node.clone();
            let flag = flag.clone();
            async move {
                if node.event.contains("m3") && flag.load(Ordering::SeqCst) {
                    Err(anyhow::anyhow!("m3 still working"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "unwound"),
            ("m3", "unwind_failed"),
            ("m4", "unwound"),
        ],
    )
    .await;

    // m1 must NOT be unwind-eligible: m3 hasn't completed unwinding
    let runnable = f.engine.runnable().await.unwrap();
    assert!(
        !runnable.iter().any(|n| n.node_id == m1),
        "m1 should not be unwind-eligible while m3 is unwind_failed"
    );

    // Retry m3's unwind
    f.engine.retry(&m3).await.unwrap();
    m3_should_fail.store(false, Ordering::SeqCst);

    // Unwind tick 3: m3 now succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "unwound"),
            ("m3", "unwound"),
            ("m4", "unwound"),
        ],
    )
    .await;

    // Unwind tick 4: m1 is now eligible (both successors m2 and m3 unwound)
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "unwound"),
            ("m2", "unwound"),
            ("m3", "unwound"),
            ("m4", "unwound"),
        ],
    )
    .await;
}

// [m1(r1) -> m2(shared) -> m3(shared)] where m2 and m3 share resource key "shared".
// Rev1 finishes fully. Rev2 deploys m1+m2, fails on m3. Unwind must process m3 before
// m2 (reverse DAG edges enforce this). The shared resource key is redundant for
// intra-pipeline ordering (edges already enforce it) but verifies no interference.
#[tokio::test]
#[traced_test]
async fn unwind_respects_shared_resource_ordering() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(shared) -> m3(shared)]
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("shared".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 1}).to_string(),
                mutates: Some("shared".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    f.engine
        .add_edge(&p1_nodes[0].node_id, &p1_nodes[1].node_id)
        .await
        .unwrap();
    f.engine
        .add_edge(&p1_nodes[1].node_id, &p1_nodes[2].node_id)
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();

    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;

    // Rev2: [m1(r1) -> m2(shared) -> m3(shared)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("shared".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 2}).to_string(),
                mutates: Some("shared".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    f.engine
        .add_edge(&p2_nodes[0].node_id, &p2_nodes[1].node_id)
        .await
        .unwrap();
    f.engine
        .add_edge(&p2_nodes[1].node_id, &p2_nodes[2].node_id)
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;
    // m2 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;
    // m3 fails (triggers auto-unwind)
    f.tick_all_fail().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "failed")],
    )
    .await;

    // Unwind tick 1: m3 (leaf)
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "unwound")],
    )
    .await;

    // Unwind tick 2: m2 (successor m3 unwound)
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "unwound"), ("m3", "unwound")],
    )
    .await;

    // Unwind tick 3: m1 (successor m2 unwound)
    f.tick_all_done().await;

    // Rev1 untouched
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;
    f.assert_states(
        &p2,
        &[("m1", "unwound"), ("m2", "unwound"), ("m3", "unwound")],
    )
    .await;
}

// [m1(r1) -> m2(r2) -> m3(r3)], three revisions.
// Rev1 finishes. Rev2 deploys m1+m2, then rev3 starts and deploys m1 (r1 is now rev3).
// Rev2 then fails on m3, triggering unwind. Since nodes stay 'finished' during unwind,
// rev3 is NOT blocked on r2 — it can deploy m2 in parallel with rev2's unwind. Rev2's
// unwind of m1 no-ops at handler level (rev3 already owns r1).
#[tokio::test]
#[traced_test]
async fn unwind_does_not_block_newer_revision() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();
    let rev3 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2) -> m3(r3)], finishes
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 1}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    f.engine
        .add_edge(&p1_nodes[0].node_id, &p1_nodes[1].node_id)
        .await
        .unwrap();
    f.engine
        .add_edge(&p1_nodes[1].node_id, &p1_nodes[2].node_id)
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;

    // Rev2: [m1(r1) -> m2(r2) -> m3(r3)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 2}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    f.engine
        .add_edge(&p2_nodes[0].node_id, &p2_nodes[1].node_id)
        .await
        .unwrap();
    f.engine
        .add_edge(&p2_nodes[1].node_id, &p2_nodes[2].node_id)
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1, m2 succeed
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;

    // Rev3 arrives: [m1(r1) -> m2(r2) -> m3(r3)]
    let p3 = f.engine.create_pipeline("web", &rev3).await.unwrap();
    let p3_m1 = f
        .engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 3}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p3_m2 = f
        .engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 3}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p3_m3 = f
        .engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 3}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&p3_m1, &p3_m2).await.unwrap();
    f.engine.add_edge(&p3_m2, &p3_m3).await.unwrap();
    f.engine.seal(&p3).await.unwrap();

    f.assert_states(
        &p3,
        &[("m1", "executable"), ("m2", "pending"), ("m3", "pending")],
    )
    .await;

    // Tick: rev2's m3 AND rev3's m1 both dispatch (different resources, both runnable).
    // Rev2's m3 fails, rev3's m1 succeeds.
    f.engine
        .tick(move |d| {
            let node = d.node.clone();
            async move {
                if node.event.contains("\"rev\":2") && node.event.contains("m3") {
                    Err(anyhow::anyhow!("rev2 m3 failure"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap()
        .join()
        .await
        .unwrap();

    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "failed")],
    )
    .await;
    f.assert_states(
        &p3,
        &[("m1", "finished"), ("m2", "executable"), ("m3", "pending")],
    )
    .await;

    // Rev3's m2 on r2: FIFO checks rev2's m2 which is 'finished' (terminal!) — NOT blocked.
    // Auto-unwind was triggered by m3's failure but nodes stay finished.
    // Rev3 can continue deploying in parallel with rev2's unwind.
    let runnable = f.engine.runnable().await.unwrap();
    assert!(
        runnable.iter().any(|n| n.node_id == p3_m2),
        "rev3's m2 should NOT be blocked — rev2's m2 is still 'finished' (terminal for FIFO)"
    );

    // Tick: rev2's m3 unwinds (restore to rev1) AND rev3's m2 deploys forward (parallel).
    // After this tick, rev3 owns r2 — so rev2's m2 becomes superseded too.
    let dispatches = f.tick_all_done().await;
    let unwind_dispatches: Vec<_> = dispatches
        .iter()
        .filter(|d| matches!(d.direction, super::engine::DispatchDirection::Unwind { .. }))
        .collect();
    assert_eq!(unwind_dispatches.len(), 1);
    Fixture::assert_unwind_dispatch(unwind_dispatches[0], "m3", Some(&rev1));
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "unwound")],
    )
    .await;
    f.assert_states(
        &p3,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "executable")],
    )
    .await;

    // Tick: rev3's m3 deploys forward. rev2's m2 and m1 are both superseded (rev3 owns
    // r1 and r2) — no more unwind dispatches for rev2.
    let dispatches = f.tick_all_done().await;
    let unwind_dispatches: Vec<_> = dispatches
        .iter()
        .filter(|d| matches!(d.direction, super::engine::DispatchDirection::Unwind { .. }))
        .collect();
    assert_eq!(unwind_dispatches.len(), 0);
    f.assert_states(
        &p3,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;

    // Final state: rev3 fully deployed, only rev2's m3 was unwound, m1+m2 stayed finished
    f.assert_states(
        &p1,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;
    f.assert_states(
        &p2,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "unwound")],
    )
    .await;
    f.assert_states(
        &p3,
        &[("m1", "finished"), ("m2", "finished"), ("m3", "finished")],
    )
    .await;
}

// [m1(r1) -> m2(r2)], two revisions.
// Rev1 finishes. Rev2 deploys m1, fails on m2. Unwind dispatches m2 (the failed node,
// leaf in scope) but the unwind handler itself returns an error. m2 becomes unwind_failed.
// m1 can never unwind (successor m2 not unwound). Pipeline is stuck — requires operator
// intervention (retry or cancel).
#[tokio::test]
#[traced_test]
async fn unwind_handler_failure_blocks_propagation() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2)], finishes
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    f.engine
        .add_edge(&p1_nodes[0].node_id, &p1_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.assert_states(&p1, &[("m1", "finished"), ("m2", "finished")])
        .await;

    // Rev2: [m1(r1) -> m2(r2)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    let p2_m1 = f
        .engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    f.engine
        .add_edge(&p2_nodes[0].node_id, &p2_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1 succeeds, m2 fails (triggers auto-unwind)
    f.tick_all_done().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "executable")])
        .await;
    f.tick_all_fail().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "failed")])
        .await;

    // Unwind tick: m2 is unwind-eligible (leaf), but handler fails
    f.tick_all_fail().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "unwind_failed")])
        .await;

    // m1 must NOT be unwind-eligible — m2's unwind didn't complete
    let runnable = f.engine.runnable().await.unwrap();
    assert!(
        !runnable.iter().any(|n| n.node_id == p2_m1),
        "m1 should not unwind while successor m2 is unwind_failed"
    );

    // Additional ticks do nothing — pipeline stuck at this point
    f.tick_all_done().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "unwind_failed")])
        .await;
}

// [m1(r1) -> m2(r2)], three revisions.
// Rev1 finishes. Rev2 deploys m1, fails on m2, unwind of m2 also fails (unwind_failed).
// Rev3 starts. Because nodes stay 'finished' during unwind (no unwind_pending), rev3's
// m1 is NOT blocked by rev2's m1 (which is still 'finished'). And rev2's m2 is
// 'unwind_failed' which is terminal for FIFO. So rev3 proceeds immediately.
#[tokio::test]
#[traced_test]
async fn newer_revision_proceeds_past_failed_unwind() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();
    let rev3 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2)], finishes
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    f.engine
        .add_edge(&p1_nodes[0].node_id, &p1_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.assert_states(&p1, &[("m1", "finished"), ("m2", "finished")])
        .await;

    // Rev2: [m1(r1) -> m2(r2)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    f.engine
        .add_edge(&p2_nodes[0].node_id, &p2_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1 succeeds, m2 fails (triggers auto-unwind)
    f.tick_all_done().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "executable")])
        .await;
    f.tick_all_fail().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "failed")])
        .await;

    // Unwind tick: m2's unwind handler also fails
    f.tick_all_fail().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "unwind_failed")])
        .await;

    // Rev3: [m1(r1) -> m2(r2)]
    let p3 = f.engine.create_pipeline("web", &rev3).await.unwrap();
    let p3_m1 = f
        .engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 3}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p3_m2 = f
        .engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 3}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&p3_m1, &p3_m2).await.unwrap();
    f.engine.seal(&p3).await.unwrap();

    f.assert_states(&p3, &[("m1", "executable"), ("m2", "pending")])
        .await;

    // Rev3's m1 on r1: rev2's m1 is 'finished' (terminal for FIFO) — NOT blocked
    let runnable = f.engine.runnable().await.unwrap();
    assert!(
        runnable.iter().any(|n| n.node_id == p3_m1),
        "rev3's m1 should not be blocked — rev2's m1 is 'finished'"
    );

    // Rev3 deploys m1
    f.tick_all_done().await;
    f.assert_states(&p3, &[("m1", "finished"), ("m2", "executable")])
        .await;

    // Rev3's m2 on r2: rev2's m2 is 'unwind_failed' (terminal for FIFO) — NOT blocked
    let runnable = f.engine.runnable().await.unwrap();
    assert!(
        runnable.iter().any(|n| n.node_id == p3_m2),
        "rev3's m2 should not be blocked — rev2's m2 is 'unwind_failed' (terminal)"
    );

    // Rev3 deploys m2
    f.tick_all_done().await;
    f.assert_states(&p3, &[("m1", "finished"), ("m2", "finished")])
        .await;
}

// [m1(r1)] single node with unwind_on_failure. m1 fails. Scope is just {m1}.
// Engine detects failure, unwinds m1 on the next tick.
#[tokio::test]
#[traced_test]
async fn unwind_single_node_pipeline() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    // [m1(r1)]
    let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(&p, &[("m1", "executable")]).await;

    // Forward: m1 fails
    f.tick_all_fail().await;
    f.assert_states(&p, &[("m1", "failed")]).await;

    // Next tick: engine detects failure, dispatches m1 for unwind
    f.tick_all_done().await;
    f.assert_states(&p, &[("m1", "unwound")]).await;
}

// [m1(r1)] single node with unwind_on_failure. m1 fails forward, then the unwind handler
// also fails. Node reaches unwind_failed. Verifies the full failure path:
// executable -> failed -> unwind_failed.
#[tokio::test]
#[traced_test]
async fn unwind_single_node_unwind_also_fails() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    // [m1(r1)]
    let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(&p, &[("m1", "executable")]).await;

    // Forward: m1 fails
    f.tick_all_fail().await;
    f.assert_states(&p, &[("m1", "failed")]).await;

    // Unwind tick: engine dispatches m1 for unwind, but that also fails
    f.tick_all_fail().await;
    f.assert_states(&p, &[("m1", "unwind_failed")]).await;

    // Further ticks do nothing — pipeline is stuck
    f.tick_all_done().await;
    f.assert_states(&p, &[("m1", "unwind_failed")]).await;
}

// [m1(r1) -> gate -> m2(r2)] where gate has is_unwind_boundary and m2 has unwind_on_failure.
// m1 and gate succeed, m2 fails. Scope walks back from m2, hits gate, stops.
// Only m2 is unwound; m1 and gate stay finished.
#[tokio::test]
#[traced_test]
async fn unwind_stops_immediately_at_gate() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    // [m1(r1) -> gate -> m2(r2)]
    let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
    let m1 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let gate = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "gate"}).to_string(),
                mutates: None,
                options: NodeOptions {
                    is_unwind_boundary: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let m2 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m2"}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&m1, &gate).await.unwrap();
    f.engine.add_edge(&gate, &m2).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    // Forward: m1, gate, m2
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("gate", "executable"),
            ("m2", "pending"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("gate", "finished"),
            ("m2", "executable"),
        ],
    )
    .await;
    f.tick_all_fail().await;
    f.assert_states(
        &p,
        &[("m1", "finished"), ("gate", "finished"), ("m2", "failed")],
    )
    .await;

    // Unwind tick: only m2 is in scope (gate blocks backward walk)
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[("m1", "finished"), ("gate", "finished"), ("m2", "unwound")],
    )
    .await;
}

// [m1(r1) -> m2(r2)] for rev2 AND rev3, both with unwind_on_failure on m2.
// Rev1 finishes. Rev2 deploys and m2 fails. Rev3 deploys and m2 fails.
// Both pipelines unwind independently without interfering.
#[tokio::test]
#[traced_test]
async fn unwind_two_pipelines_concurrently() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();
    let rev3 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2)], finishes
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    f.engine
        .add_edge(&p1_nodes[0].node_id, &p1_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p1).await.unwrap();
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.assert_states(&p1, &[("m1", "finished"), ("m2", "finished")])
        .await;

    // Rev2: [m1(r1) -> m2(r2, unwind_on_failure)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    f.engine
        .add_edge(&p2_nodes[0].node_id, &p2_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1 succeeds, m2 fails
    f.tick_all_done().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "executable")])
        .await;
    f.tick_all_fail().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "failed")])
        .await;

    // Rev3: [m1(r1) -> m2(r2, unwind_on_failure)]
    let p3 = f.engine.create_pipeline("web", &rev3).await.unwrap();
    f.engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 3}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p3,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 3}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p3_nodes = f.engine.nodes(&p3).await.unwrap();
    f.engine
        .add_edge(&p3_nodes[0].node_id, &p3_nodes[1].node_id)
        .await
        .unwrap();
    f.engine.seal(&p3).await.unwrap();

    // This tick: detect_and_trigger_unwind fires for p2 (m2 failed with unwind_on_failure).
    // p2 → unwinding. Then dispatch: rev3's m1 (forward, unblocked) AND rev2's m2 (unwind,
    // leaf in scope). Both succeed.
    f.tick_all_done().await;
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "unwound")])
        .await;
    f.assert_states(&p3, &[("m1", "finished"), ("m2", "executable")])
        .await;

    // Next tick: rev3's m2 is FIFO-unblocked (rev2's m2 is 'unwound' = terminal).
    // rev2's m1 is NOT dispatched — superseded by rev3 on r1 (rev3's m1 is finished).
    // Rev3's m2 fails forward.
    f.engine
        .tick(move |d| {
            let node = d.node.clone();
            async move {
                if node.event.contains("\"rev\":3") && node.event.contains("m2") {
                    Err(anyhow::anyhow!("rev3 m2 failure"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    f.assert_states(&p2, &[("m1", "finished"), ("m2", "unwound")])
        .await;
    f.assert_states(&p3, &[("m1", "finished"), ("m2", "failed")])
        .await;

    // Now rev3 is ALSO unwinding (m2 has unwind_on_failure). Rev3's m2 unwinds.
    // Rev3's m1 is the only finished node on r1 — NOT superseded — so it will unwind.
    f.tick_all_done().await;
    f.assert_states(&p3, &[("m1", "finished"), ("m2", "unwound")])
        .await;

    // Rev3's m1 unwinds (no newer revision on r1 — rev3 is the newest).
    f.tick_all_done().await;
    f.assert_states(&p3, &[("m1", "unwound"), ("m2", "unwound")])
        .await;
}

// [m1(r1) -> m2(r2) -> m3(r3) -> m4(r4)], all nodes have unwind_on_failure.
// Rev1 finishes. Rev2 deploys m1+m2, fails on m3. m4 was never reached (pending).
// Unwind scope walks back from m3: {m3, m2, m1}. Forward cascade from m1 reaches m4
// (pending) — it gets cancelled. Final state: m1-m3 unwound, m4 cancelled.
#[tokio::test]
#[traced_test]
async fn unwind_mid_pipeline_leaves_unreached_nodes_pending() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    // Rev1: [m1(r1) -> m2(r2) -> m3(r3) -> m4(r4)], finishes
    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 1}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 1}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 1}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "m4", "rev": 1}).to_string(),
                mutates: Some("r4".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p1_nodes = f.engine.nodes(&p1).await.unwrap();
    for i in 0..3 {
        f.engine
            .add_edge(&p1_nodes[i].node_id, &p1_nodes[i + 1].node_id)
            .await
            .unwrap();
    }
    f.engine.seal(&p1).await.unwrap();

    // Rev1 finishes all 4 nodes
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[
            ("m1", "finished"),
            ("m2", "executable"),
            ("m3", "pending"),
            ("m4", "pending"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "executable"),
            ("m4", "pending"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "finished"),
            ("m4", "executable"),
        ],
    )
    .await;
    f.tick_all_done().await;
    f.assert_states(
        &p1,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "finished"),
            ("m4", "finished"),
        ],
    )
    .await;

    // Rev2: [m1(r1) -> m2(r2) -> m3(r3) -> m4(r4)]
    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m1", "rev": 2}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m2", "rev": 2}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m3", "rev": 2}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    f.engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "m4", "rev": 2}).to_string(),
                mutates: Some("r4".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let p2_nodes = f.engine.nodes(&p2).await.unwrap();
    for i in 0..3 {
        f.engine
            .add_edge(&p2_nodes[i].node_id, &p2_nodes[i + 1].node_id)
            .await
            .unwrap();
    }
    f.engine.seal(&p2).await.unwrap();

    // Rev2 forward: m1 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "executable"),
            ("m3", "pending"),
            ("m4", "pending"),
        ],
    )
    .await;

    // Rev2 forward: m2 succeeds
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "executable"),
            ("m4", "pending"),
        ],
    )
    .await;

    // Rev2 forward: m3 fails (triggers auto-unwind)
    f.tick_all_fail().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "failed"),
            ("m4", "pending"),
        ],
    )
    .await;

    // Unwind tick 1: m3 is leaf in scope. m4 was pending — forward cascade cancels it.
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "unwound"),
            ("m4", "cancelled"),
        ],
    )
    .await;

    // Unwind tick 2: m2 (successor m3 is unwound)
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "finished"),
            ("m2", "unwound"),
            ("m3", "unwound"),
            ("m4", "cancelled"),
        ],
    )
    .await;

    // Unwind tick 3: m1 (successor m2 is unwound)
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "unwound"),
            ("m2", "unwound"),
            ("m3", "unwound"),
            ("m4", "cancelled"),
        ],
    )
    .await;

    // Additional ticks do nothing — m4 stays cancelled
    f.tick_all_done().await;
    f.assert_states(
        &p2,
        &[
            ("m1", "unwound"),
            ("m2", "unwound"),
            ("m3", "unwound"),
            ("m4", "cancelled"),
        ],
    )
    .await;
}

// [m1(r1) -> [m2(r2, unwind_on_failure) -> m3(r3), m4(r4) -> m5(r5)]]
// Fan-out from m1 into two branches. m2 fails, triggering unwind.
// Backward scope from m2: {m2, m1}. But m1 is in scope, and m4+m5 depend on m1.
// Forward cascade: m4 (finished) joins scope, m5 (executable) gets cancelled.
// m3 is pending (never reached) — gets cancelled too.
// Final state: m1, m2, m4 unwound. m3, m5 cancelled.
#[tokio::test]
#[traced_test]
async fn unwind_cascades_forward_through_finished_dependents() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();

    // [m1(r1) -> [m2(r2, unwind_on_failure) -> m3(r3), m4(r4) -> m5(r5)]]
    let p = f.engine.create_pipeline("web", &rev1).await.unwrap();
    let m1 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: Some("r1".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m2 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m2"}).to_string(),
                mutates: Some("r2".to_string()),
                options: NodeOptions {
                    unwind_on_failure: true,
                    ..NodeOptions::default()
                },
            },
        )
        .await
        .unwrap();
    let m3 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m3"}).to_string(),
                mutates: Some("r3".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m4 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m4"}).to_string(),
                mutates: Some("r4".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m5 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m5"}).to_string(),
                mutates: Some("r5".to_string()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    f.engine.add_edge(&m1, &m2).await.unwrap();
    f.engine.add_edge(&m2, &m3).await.unwrap();
    f.engine.add_edge(&m1, &m4).await.unwrap();
    f.engine.add_edge(&m4, &m5).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(
        &p,
        &[
            ("m1", "executable"),
            ("m2", "pending"),
            ("m3", "pending"),
            ("m4", "pending"),
            ("m5", "pending"),
        ],
    )
    .await;

    // Forward: m1 succeeds, promotes m2 and m4
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "executable"),
            ("m3", "pending"),
            ("m4", "executable"),
            ("m5", "pending"),
        ],
    )
    .await;

    // Forward: m2 and m4 both dispatch. m4 succeeds, m2 fails (triggers unwind).
    f.engine
        .tick(move |d| {
            let node = d.node.clone();
            async move {
                if node.event.contains("m2") {
                    Err(anyhow::anyhow!("m2 failure"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "failed"),
            ("m3", "pending"),
            ("m4", "finished"),
            ("m5", "executable"),
        ],
    )
    .await;

    // Unwind tick 1: auto-unwind triggers. Scope computation:
    //   Backward from m2: {m2, m1}
    //   Forward cascade from m1: m4 (finished) joins scope.
    //   Forward cascade from m4: m5 (executable) → cancelled.
    //   Forward cascade from m2: m3 (pending) → cancelled.
    // After scope computation: m3 and m5 are cancelled (never ran).
    // Unwind-eligible: m2 (successor m3 cancelled) AND m4 (successor m5 cancelled) — parallel.
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "unwound"),
            ("m3", "cancelled"),
            ("m4", "unwound"),
            ("m5", "cancelled"),
        ],
    )
    .await;

    // Unwind tick 2: m1 is eligible (successors m2 and m4 both unwound)
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "unwound"),
            ("m2", "unwound"),
            ("m3", "cancelled"),
            ("m4", "unwound"),
            ("m5", "cancelled"),
        ],
    )
    .await;

    // Additional ticks do nothing
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "unwound"),
            ("m2", "unwound"),
            ("m3", "cancelled"),
            ("m4", "unwound"),
            ("m5", "cancelled"),
        ],
    )
    .await;
}

// [m1 -> [m2, m3] -> [m4, m5]] — wave structure with many-to-many edges between layers.
// Wave 1 = {m2, m3}, wave 2 = {m4, m5}. Each node in wave 2 depends on ALL of wave 1.
// Tests that wave 2 doesn't start until the entire wave 1 is done, regardless of which
// wave 1 node finishes first. Uses a gate to make m3 finish one tick after m2.
#[tokio::test]
#[traced_test]
async fn pipeline_engine_wave_barrier_waits_for_all_predecessors() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let f = Fixture::new().await;
    let rev = f.db.create_revision("web").await.unwrap();

    // [m1 -> [m2, m3] -> [m4, m5]]
    let p = f.engine.create_pipeline("web", &rev).await.unwrap();
    let m1 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m1"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m2 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m2"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m3 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m3"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m4 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m4"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let m5 = f
        .engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "m5"}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    // m1 -> wave 1
    f.engine.add_edge(&m1, &m2).await.unwrap();
    f.engine.add_edge(&m1, &m3).await.unwrap();
    // wave 1 -> wave 2 (all-to-all)
    f.engine.add_edge(&m2, &m4).await.unwrap();
    f.engine.add_edge(&m2, &m5).await.unwrap();
    f.engine.add_edge(&m3, &m4).await.unwrap();
    f.engine.add_edge(&m3, &m5).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    f.assert_states(
        &p,
        &[
            ("m1", "executable"),
            ("m2", "pending"),
            ("m3", "pending"),
            ("m4", "pending"),
            ("m5", "pending"),
        ],
    )
    .await;

    // Tick 1: m1 succeeds, wave 1 promoted
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "executable"),
            ("m3", "executable"),
            ("m4", "pending"),
            ("m5", "pending"),
        ],
    )
    .await;

    // Tick 2: m2 succeeds, m3 fails (simulating m3 taking longer).
    // Wave 2 must NOT be promoted — m3 is still not finished.
    let m3_should_fail = Arc::new(AtomicBool::new(true));
    let flag = m3_should_fail.clone();
    f.engine
        .tick(move |d| {
            let node = d.node.clone();
            let flag = flag.clone();
            async move {
                if node.event.contains("m3") && flag.load(Ordering::SeqCst) {
                    Err(anyhow::anyhow!("m3 slow"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "failed"),
            ("m4", "pending"),
            ("m5", "pending"),
        ],
    )
    .await;

    // m4 and m5 must NOT be executable — m3 (a predecessor) is failed, not finished
    let runnable = f.engine.runnable().await.unwrap();
    assert!(
        !runnable.iter().any(|n| n.node_id == m4 || n.node_id == m5),
        "wave 2 should not be runnable while m3 is failed"
    );

    // Retry m3
    m3_should_fail.store(false, Ordering::SeqCst);
    f.engine.retry(&m3).await.unwrap();
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "executable"),
            ("m4", "pending"),
            ("m5", "pending"),
        ],
    )
    .await;

    // Tick 3: m3 succeeds. Wave 2 is now promotable (both predecessors finished).
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "finished"),
            ("m4", "executable"),
            ("m5", "executable"),
        ],
    )
    .await;

    // Tick 4: wave 2 executes in parallel
    f.tick_all_done().await;
    f.assert_states(
        &p,
        &[
            ("m1", "finished"),
            ("m2", "finished"),
            ("m3", "finished"),
            ("m4", "finished"),
            ("m5", "finished"),
        ],
    )
    .await;
}
