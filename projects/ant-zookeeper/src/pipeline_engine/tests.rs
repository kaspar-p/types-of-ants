use ant_library::db::TypesOfAntsDatabase;
use ant_library_test::db::TestDatabase;
use ant_zookeeper_db::AntZooStorageClient;
use tracing_test::traced_test;

use super::engine::{BlockReason, Edge, Job, Node, Pipeline, PipelineEngine};
use super::node::NodeOptions;

use crate::pipeline::resource_key::{DeploymentResource, Identifier};

fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}

mod nodes {
    use super::*;

    pub fn host_replicated(host_id: &str, service_id: &str) -> NodeOptions {
        let resource = DeploymentResource::HostService {
            host_id: id(host_id),
            service_id: id(service_id),
        };
        NodeOptions {
            event: serde_json::json!({
                "type": "host-replicated",
                "host_id": host_id,
                "service_id": service_id,
            })
            .to_string(),
            mutates: Some(resource.to_string()),
        }
    }

    pub fn host_deployed(host_id: &str, service_id: &str) -> NodeOptions {
        let resource = DeploymentResource::HostService {
            host_id: id(host_id),
            service_id: id(service_id),
        };
        NodeOptions {
            event: serde_json::json!({
                "type": "host-deployed",
                "host_id": host_id,
                "service_id": service_id,
            })
            .to_string(),
            mutates: Some(resource.to_string()),
        }
    }

    pub fn synthetic(name: &str) -> NodeOptions {
        NodeOptions {
            event: serde_json::json!({ "type": name }).to_string(),
            mutates: None,
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

    async fn tick_all_done(&self) {
        self.engine
            .tick(|_| async { Ok(()) })
            .await
            .unwrap()
            .join()
            .await
            .unwrap();
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
        .add_node(&p, nodes::host_replicated("w1", "web"))
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
        let start = f.engine.add_node(&p, nodes::synthetic("start")).await.unwrap();
        let n1 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "replicate", "host": "w1"}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
        let n2 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "deploy", "host": "w1"}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
        let n3 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "verify", "host": "w1"}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
        let end = f.engine.add_node(&p, nodes::synthetic("end")).await.unwrap();
        f.engine.add_edge(&start, &n1).await.unwrap();
        f.engine.add_edge(&n1, &n2).await.unwrap();
        f.engine.add_edge(&n2, &n3).await.unwrap();
        f.engine.add_edge(&n3, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        p
    };

    // [rev1, start] succeeds
    f.tick_all_done().await;

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("replicate", "executable"),
        ("deploy", "pending"),
        ("verify", "pending"),
        ("end", "pending"),
    ]).await;

    // [rev1, node1] succeeds
    f.tick_all_done().await;

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("replicate", "finished"),
        ("deploy", "executable"),
        ("verify", "pending"),
        ("end", "pending"),
    ]).await;

    // [rev1, node2] succeeds
    f.tick_all_done().await;

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("replicate", "finished"),
        ("deploy", "finished"),
        ("verify", "executable"),
        ("end", "pending"),
    ]).await;

    // [rev2, start] — rev2 arrives mid-pipeline while rev1 is between node2 and node3
    let rev2 = f.db.create_revision("web").await.unwrap();
    let p2 = {
        let p = f.engine.create_pipeline("web", &rev2).await.unwrap();
        let start = f.engine.add_node(&p, nodes::synthetic("start")).await.unwrap();
        let n1 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "replicate", "host": "w1"}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
        let n2 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "deploy", "host": "w1"}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
        let n3 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "verify", "host": "w1"}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
        let end = f.engine.add_node(&p, nodes::synthetic("end")).await.unwrap();
        f.engine.add_edge(&start, &n1).await.unwrap();
        f.engine.add_edge(&n1, &n2).await.unwrap();
        f.engine.add_edge(&n2, &n3).await.unwrap();
        f.engine.add_edge(&n3, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        p
    };

    // Rev2 exists — start is runnable (no resource key), but node1 is FIFO-blocked
    f.assert_states(&p2, &[
        ("start", "executable"),
        ("replicate", "pending"),
        ("deploy", "pending"),
        ("verify", "pending"),
        ("end", "pending"),
    ]).await;

    // Only rev1's verify and rev2's start are runnable
    let runnable = f.engine.runnable().await.unwrap();
    assert_eq!(runnable.len(), 2);

    // [rev1, node3] starts as long-running, [rev2, start] also runs (no resource contention)
    let gate = Arc::new(Notify::new());

    let tick_handle = {
        let gate = gate.clone();
        f.engine.tick(move |node| {
            let gate = gate.clone();
            async move {
                if node.event.contains("verify") {
                    gate.notified().await;
                }
                Ok(())
            }
        }).await.unwrap()
    };

    // [rev1, node3] completes → [rev1, end] promoted
    gate.notify_one();
    tick_handle.join().await.unwrap();

    // After join: rev1 verify finished, rev2 start finished (both dispatched in same tick)
    f.assert_states(&p1, &[
        ("start", "finished"),
        ("replicate", "finished"),
        ("deploy", "finished"),
        ("verify", "finished"),
        ("end", "executable"),
    ]).await;

    f.assert_states(&p2, &[
        ("start", "finished"),
        ("replicate", "executable"),
        ("deploy", "pending"),
        ("verify", "pending"),
        ("end", "pending"),
    ]).await;

    // [rev1, end] completes — rev1 fully done
    f.tick_all_done().await;

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("replicate", "finished"),
        ("deploy", "finished"),
        ("verify", "finished"),
        ("end", "finished"),
    ]).await;

    // [rev2, node1] [rev2, node2] [rev2, node3] [rev2, end] — now unblocked
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.tick_all_done().await;
    f.tick_all_done().await;

    f.assert_states(&p2, &[
        ("start", "finished"),
        ("replicate", "finished"),
        ("deploy", "finished"),
        ("verify", "finished"),
        ("end", "finished"),
    ]).await;
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
            let n = f.engine.add_node(&p, NodeOptions {
                event: serde_json::json!({"type": "predecessor", "i": i, "iter": iteration}).to_string(),
                mutates: None,
            }).await.unwrap();
            predecessors.push(n);
        }
        let end = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "join", "iter": iteration}).to_string(),
            mutates: None,
        }).await.unwrap();
        for pred in &predecessors {
            f.engine.add_edge(pred, &end).await.unwrap();
        }
        f.engine.seal(&p).await.unwrap();

        let barrier = Arc::new(Barrier::new(10));

        let tick_handle = {
            let barrier = barrier.clone();
            f.engine.tick(move |_node| {
                let barrier = barrier.clone();
                async move {
                    barrier.wait().await;
                    Ok(())
                }
            }).await.unwrap()
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
        assert_eq!(end_node.state, "finished", "iteration {iteration}: fan-in node not finished after tick");
    }
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_stress_concurrent_ticks_no_double_dispatch() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
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
        f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "work", "rev": rev}).to_string(),
            mutates: Some(resource.to_string()),
        }).await.unwrap();
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

                let tick_handle = engine.tick(move |_node| {
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
                }).await.unwrap();

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
    let a = f.engine.add_node(&p, nodes::synthetic("start")).await.unwrap();
    let b = f.engine.add_node(&p, nodes::host_replicated("w1", "web")).await.unwrap();
    let c = f.engine.add_node(&p, nodes::host_replicated("w2", "web")).await.unwrap();
    let d = f.engine.add_node(&p, nodes::synthetic("end")).await.unwrap();
    f.engine.add_edge(&a, &b).await.unwrap();
    f.engine.add_edge(&a, &c).await.unwrap();
    f.engine.add_edge(&b, &d).await.unwrap();
    f.engine.add_edge(&c, &d).await.unwrap();
    f.engine.seal(&p).await.unwrap();

    let edges = f.engine.edges(&p).await.unwrap();
    assert_eq!(edges.len(), 4);

    let pairs: Vec<(&str, &str)> = edges.iter().map(|e| (e.from_node_id.as_str(), e.to_node_id.as_str())).collect();
    assert!(pairs.contains(&(a.as_str(), b.as_str())));
    assert!(pairs.contains(&(a.as_str(), c.as_str())));
    assert!(pairs.contains(&(b.as_str(), d.as_str())));
    assert!(pairs.contains(&(c.as_str(), d.as_str())));
}

#[tokio::test]
#[traced_test]
async fn pipeline_engine_active_pipelines_returns_in_progress() {
    let f = Fixture::new().await;
    let rev1 = f.db.create_revision("web").await.unwrap();
    let rev2 = f.db.create_revision("web").await.unwrap();

    let p1 = f.engine.create_pipeline("web", &rev1).await.unwrap();
    f.engine.add_node(&p1, nodes::synthetic("start")).await.unwrap();
    f.engine.seal(&p1).await.unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine.add_node(&p2, nodes::synthetic("start")).await.unwrap();
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
    f.engine.add_node(&p1, nodes::host_replicated("w1", "web")).await.unwrap();
    f.engine.seal(&p1).await.unwrap();

    let p2 = f.engine.create_pipeline("web", &rev2).await.unwrap();
    f.engine.add_node(&p2, nodes::host_replicated("w1", "web")).await.unwrap();
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
    let n = f.engine.add_node(&p, nodes::host_replicated("w1", "web")).await.unwrap();
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
        let start = f.engine.add_node(&p, nodes::synthetic("start")).await.unwrap();
        let m1 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "middle", "name": "m1"}).to_string(),
            mutates: Some("m1-resource".to_string()),
        }).await.unwrap();
        let m2 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "middle", "name": "m2"}).to_string(),
            mutates: Some("m2-resource".to_string()),
        }).await.unwrap();
        let end = f.engine.add_node(&p, nodes::synthetic("end")).await.unwrap();
        f.engine.add_edge(&start, &m1).await.unwrap();
        f.engine.add_edge(&start, &m2).await.unwrap();
        f.engine.add_edge(&m1, &end).await.unwrap();
        f.engine.add_edge(&m2, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        (p, m1, m2)
    };

    f.tick_all_done().await;

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("m1", "executable"),
        ("m2", "executable"),
        ("end", "pending"),
    ]).await;

    let m1_gate = Arc::new(Notify::new());
    let m2_gate = Arc::new(Notify::new());

    let tick_handle = {
        let m1_gate = m1_gate.clone();
        let m2_gate = m2_gate.clone();
        f.engine.tick(move |node| {
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
        }).await.unwrap()
    };

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("m1", "in_progress"),
        ("m2", "in_progress"),
        ("end", "pending"),
    ]).await;

    assert_eq!(f.engine.node_job(&m1).await.unwrap().unwrap().state, "running");
    assert_eq!(f.engine.node_job(&m2).await.unwrap().unwrap().state, "running");

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].pipeline_id, p1);

    let rev2 = f.db.create_revision("web").await.unwrap();
    let p2 = {
        let p = f.engine.create_pipeline("web", &rev2).await.unwrap();
        let start = f.engine.add_node(&p, nodes::synthetic("start")).await.unwrap();
        let m1 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "middle", "name": "m1"}).to_string(),
            mutates: Some("m1-resource".to_string()),
        }).await.unwrap();
        let m2 = f.engine.add_node(&p, NodeOptions {
            event: serde_json::json!({"type": "middle", "name": "m2"}).to_string(),
            mutates: Some("m2-resource".to_string()),
        }).await.unwrap();
        let end = f.engine.add_node(&p, nodes::synthetic("end")).await.unwrap();
        f.engine.add_edge(&start, &m1).await.unwrap();
        f.engine.add_edge(&start, &m2).await.unwrap();
        f.engine.add_edge(&m1, &end).await.unwrap();
        f.engine.add_edge(&m2, &end).await.unwrap();
        f.engine.seal(&p).await.unwrap();
        p
    };

    f.tick_all_done().await;

    f.assert_states(&p2, &[
        ("start", "finished"),
        ("m1", "executable"),
        ("m2", "executable"),
        ("end", "pending"),
    ]).await;

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

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("m1", "failed"),
        ("m2", "failed"),
        ("end", "pending"),
    ]).await;

    f.engine.cancel(&p1).await.unwrap();

    f.assert_states(&p1, &[
        ("start", "finished"),
        ("m1", "cancelled"),
        ("m2", "cancelled"),
        ("end", "cancelled"),
    ]).await;

    let m1_gate2 = Arc::new(Notify::new());
    let m2_gate2 = Arc::new(Notify::new());

    let tick_handle2 = {
        let m1_gate = m1_gate2.clone();
        let m2_gate = m2_gate2.clone();
        f.engine.tick(move |node| {
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
        }).await.unwrap()
    };

    f.assert_states(&p2, &[
        ("start", "finished"),
        ("m1", "in_progress"),
        ("m2", "in_progress"),
        ("end", "pending"),
    ]).await;

    m1_gate2.notify_one();
    m2_gate2.notify_one();
    tick_handle2.join().await.unwrap();

    // Next tick promotes end (may be pending due to concurrent succeed() race) and executes it
    f.tick_all_done().await;

    f.assert_states(&p2, &[
        ("start", "finished"),
        ("m1", "finished"),
        ("m2", "finished"),
        ("end", "finished"),
    ]).await;

    let latest = f.engine.latest_finished_pipeline("web").await.unwrap().unwrap();
    assert_eq!(latest.pipeline_id, p2);

    let active = f.engine.active_pipelines("web").await.unwrap();
    assert_eq!(active.len(), 0);
}
