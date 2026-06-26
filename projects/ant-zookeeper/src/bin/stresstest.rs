use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use ant_library::db::TypesOfAntsDatabase;
use ant_library_test::db::TestDatabase;
use ant_zookeeper::pipeline_engine::engine::PipelineEngine;
use ant_zookeeper::pipeline_engine::node::{NodeOptions, NodeSpec};
use ant_zookeeper_db::AntZooStorageClient;
use tokio::sync::Barrier;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let run_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let output_dir = format!("stress-runs/{run_id}");
    std::fs::create_dir_all(&output_dir).unwrap();

    tracing::info!("stress test run_id={run_id}, output={output_dir}");

    std::panic::set_hook({
        let output_dir = output_dir.clone();
        Box::new(move |info| {
            eprintln!("PANIC: {info}");
            eprintln!("Output dir: {output_dir}");
        })
    });

    let guard = TestDatabase::new("ant-zookeeper-db").await;
    let db = AntZooStorageClient::connect(&guard.config).await.unwrap();
    let engine = PipelineEngine::new(db.pool()).await.unwrap();

    let total_dispatches = Arc::new(AtomicU32::new(0));

    let mut iter: u64 = 0;
    loop {
        if iter % 100 == 0 {
            tracing::info!(
                "iteration={iter} dispatches={}",
                total_dispatches.load(Ordering::SeqCst)
            );
        }

        run_scenario_concurrent_fifo(&engine, &db, iter, &total_dispatches).await;
        run_scenario_fan_in(&engine, &db, iter, &total_dispatches).await;
        run_scenario_mixed_resources(&engine, &db, iter, &total_dispatches).await;

        iter += 1;
    }
}

async fn run_scenario_concurrent_fifo(
    engine: &PipelineEngine,
    db: &AntZooStorageClient,
    iter: u64,
    dispatches: &Arc<AtomicU32>,
) {
    let rev1 = db.create_revision("ant-on-the-web").await.unwrap();
    let rev2 = db.create_revision("ant-on-the-web").await.unwrap();

    let resource = format!("fifo-resource-{iter}");

    let p1 = engine
        .create_pipeline("ant-on-the-web", &rev1)
        .await
        .unwrap();
    engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "work", "iter": iter, "rev": 1}).to_string(),
                mutates: Some(resource.clone()),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    engine.seal(&p1).await.unwrap();

    let p2 = engine
        .create_pipeline("ant-on-the-web", &rev2)
        .await
        .unwrap();
    engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "work", "iter": iter, "rev": 2}).to_string(),
                mutates: Some(resource),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    engine.seal(&p2).await.unwrap();

    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    for _ in 0..5 {
        let pool = db.pool();
        let barrier = barrier.clone();
        let dispatches = dispatches.clone();

        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let engine = PipelineEngine::new(pool).await.unwrap();
            let tick_handle = engine
                .tick(move |_| {
                    let d = dispatches.clone();
                    async move {
                        d.fetch_add(1, Ordering::SeqCst);
                        tokio::task::yield_now().await;
                        Ok(())
                    }
                })
                .await
                .unwrap();
            tick_handle.join().await.unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // Second round for rev2
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    for _ in 0..5 {
        let pool = db.pool();
        let barrier = barrier.clone();
        let dispatches = dispatches.clone();

        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let engine = PipelineEngine::new(pool).await.unwrap();
            let tick_handle = engine
                .tick(move |_| {
                    let d = dispatches.clone();
                    async move {
                        d.fetch_add(1, Ordering::SeqCst);
                        tokio::task::yield_now().await;
                        Ok(())
                    }
                })
                .await
                .unwrap();
            tick_handle.join().await.unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

async fn run_scenario_fan_in(
    engine: &PipelineEngine,
    db: &AntZooStorageClient,
    iter: u64,
    dispatches: &Arc<AtomicU32>,
) {
    let rev = db.create_revision("ant-on-the-web").await.unwrap();
    let p = engine
        .create_pipeline("ant-on-the-web", &rev)
        .await
        .unwrap();

    let mut preds = vec![];
    for i in 0..8 {
        let n = engine
            .add_node(
                &p,
                NodeSpec {
                    event: serde_json::json!({"type": "fan_in_pred", "iter": iter, "i": i})
                        .to_string(),
                    mutates: None,
                    options: NodeOptions::default(),
                },
            )
            .await
            .unwrap();
        preds.push(n);
    }

    let join = engine
        .add_node(
            &p,
            NodeSpec {
                event: serde_json::json!({"type": "fan_in_join", "iter": iter}).to_string(),
                mutates: None,
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();

    for pred in &preds {
        engine.add_edge(pred, &join).await.unwrap();
    }
    engine.seal(&p).await.unwrap();

    let barrier = Arc::new(Barrier::new(8));
    let dispatches_clone = dispatches.clone();

    let tick_handle = engine
        .tick(move |_| {
            let barrier = barrier.clone();
            let d = dispatches_clone.clone();
            async move {
                d.fetch_add(1, Ordering::SeqCst);
                barrier.wait().await;
                Ok(())
            }
        })
        .await
        .unwrap();

    tick_handle.join().await.unwrap();

    // Join node should be promotable after next tick
    let tick_handle = engine.tick(move |_| async move { Ok(()) }).await.unwrap();
    tick_handle.join().await.unwrap();

    let nodes = engine.nodes(&p).await.unwrap();
    let join_node = nodes
        .iter()
        .find(|n| n.event.contains("fan_in_join"))
        .unwrap();
    assert!(
        join_node.state == "finished",
        "iter={iter}: fan-in join not finished, state={}",
        join_node.state
    );
}

async fn run_scenario_mixed_resources(
    engine: &PipelineEngine,
    db: &AntZooStorageClient,
    iter: u64,
    dispatches: &Arc<AtomicU32>,
) {
    let rev1 = db.create_revision("ant-on-the-web").await.unwrap();
    let rev2 = db.create_revision("ant-gateway").await.unwrap();

    let p1 = engine
        .create_pipeline("ant-on-the-web", &rev1)
        .await
        .unwrap();
    let a = engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "mixed_a", "iter": iter}).to_string(),
                mutates: Some(format!("res-a-{iter}")),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    let b = engine
        .add_node(
            &p1,
            NodeSpec {
                event: serde_json::json!({"type": "mixed_b", "iter": iter}).to_string(),
                mutates: Some(format!("res-b-{iter}")),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    engine.seal(&p1).await.unwrap();

    let p2 = engine.create_pipeline("ant-gateway", &rev2).await.unwrap();
    engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "mixed_c", "iter": iter}).to_string(),
                mutates: Some(format!("res-a-{iter}")),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    engine
        .add_node(
            &p2,
            NodeSpec {
                event: serde_json::json!({"type": "mixed_d", "iter": iter}).to_string(),
                mutates: Some(format!("res-b-{iter}")),
                options: NodeOptions::default(),
            },
        )
        .await
        .unwrap();
    engine.seal(&p2).await.unwrap();

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let pool = db.pool();
        let barrier = barrier.clone();
        let dispatches = dispatches.clone();

        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let engine = PipelineEngine::new(pool).await.unwrap();
            let tick_handle = engine
                .tick(move |_| {
                    let d = dispatches.clone();
                    async move {
                        d.fetch_add(1, Ordering::SeqCst);
                        tokio::task::yield_now().await;
                        Ok(())
                    }
                })
                .await
                .unwrap();
            tick_handle.join().await.unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // Second round
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let pool = db.pool();
        let barrier = barrier.clone();
        let dispatches = dispatches.clone();

        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let engine = PipelineEngine::new(pool).await.unwrap();
            let tick_handle = engine
                .tick(move |_| {
                    let d = dispatches.clone();
                    async move {
                        d.fetch_add(1, Ordering::SeqCst);
                        tokio::task::yield_now().await;
                        Ok(())
                    }
                })
                .await
                .unwrap();
            tick_handle.join().await.unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let nodes1 = engine.nodes(&p1).await.unwrap();
    let nodes2 = engine.nodes(&p2).await.unwrap();

    for n in nodes1.iter().chain(nodes2.iter()) {
        assert_eq!(
            n.state, "finished",
            "iter={iter}: node {} not finished, state={}",
            n.event, n.state
        );
    }
}
