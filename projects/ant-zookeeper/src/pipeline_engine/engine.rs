use ant_library::db::ConnectionPool;
use anyhow::Context;
use stdext::function_name;
use tracing::{error, info, info_span, warn, Instrument};

use super::node::NodeOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub pipeline_id: String,
    pub project_id: String,
    pub revision_id: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub job_id: String,
    pub node_id: String,
    pub state: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub node_id: String,
    pub event: String,
    pub state: String,
    pub resource_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    /// Node is not in 'executable' state (e.g. pending, failed, in_progress).
    /// If pending, includes the unfinished predecessors blocking promotion.
    NotExecutable {
        state: String,
        pending_predecessors: Vec<Node>,
    },
    /// Node is executable but an older revision has incomplete work on the same resource.
    ResourceContention {
        blocking_node: Node,
        blocking_revision: String,
    },
}

/// Handle given to a spawned task to report job completion.
/// Calling `succeed()` or `fail()` atomically updates both the job and the node.
#[derive(Clone)]
pub struct JobHandle {
    db: ConnectionPool,
    job_id: String,
    node_id: String,
}

impl JobHandle {
    pub async fn succeed(&self) -> Result<(), anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let updated = tx
            .execute(
                "
                update pipeline_engine_job
                set
                    state = 'succeeded',
                    finished_at = now(),
                    updated_at = now()
                where
                    job_id = $1
                    and state = 'running'
                ",
                &[&self.job_id],
            )
            .await
            .with_context(|| format!("{}: job={}", function_name!(), self.job_id))?;

        if updated == 0 {
            tx.rollback().await?;
            warn!(job_id = %self.job_id, "succeed called on non-running job, ignoring");
            return Ok(());
        }

        let node_updated = tx
            .execute(
                "
                update pipeline_engine_node
                set
                    state = 'finished',
                    finished_at = now(),
                    updated_at = now()
                where
                    node_id = $1
                    and state = 'in_progress'
                ",
                &[&self.node_id],
            )
            .await
            .with_context(|| format!("{}: node={}", function_name!(), self.node_id))?;

        assert_node_was_in_progress("succeed", &self.node_id, node_updated);

        // Promote successors whose predecessors are all done
        tx.execute(
            "
            update pipeline_engine_node
            set
                state = 'executable',
                updated_at = now()
            where
                state = 'pending'
                and node_id in (
                    select e.to_node_id
                    from pipeline_engine_edge e
                    where e.from_node_id = $1
                )
                and not exists (
                    select 1
                    from pipeline_engine_edge e2
                        join pipeline_engine_node pred
                            on e2.from_node_id = pred.node_id
                    where
                        e2.to_node_id = pipeline_engine_node.node_id
                        and pred.state not in ('finished', 'cancelled')
                )
            ",
            &[&self.node_id],
        )
        .await
        .with_context(|| format!("{}: promote node={}", function_name!(), self.node_id))?;

        // If all nodes in pipeline are terminal, mark pipeline finished
        let pipeline_id: String = tx
            .query_one(
                "
                select pipeline_id
                from pipeline_engine_node
                where node_id = $1
                ",
                &[&self.node_id],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: lookup pipeline node={}",
                    function_name!(),
                    self.node_id
                )
            })?
            .get("pipeline_id");

        let remaining: i64 = tx
            .query_one(
                "
                select count(*) as c
                from pipeline_engine_node
                where
                    pipeline_id = $1
                    and state not in ('finished', 'cancelled')
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: count remaining pipeline={}",
                    function_name!(),
                    pipeline_id
                )
            })?
            .get("c");

        if remaining == 0 {
            tx.execute(
                "
                update pipeline_engine_pipeline
                set
                    state = 'finished',
                    finished_at = now(),
                    updated_at = now()
                where
                    pipeline_id = $1
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: finalize pipeline={}", function_name!(), pipeline_id))?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn heartbeat(&self) -> Result<(), anyhow::Error> {
        let con = self.db.get().await?;

        con.execute(
            "
            update pipeline_engine_job
            set
                last_heartbeat_at = now(),
                updated_at = now()
            where
                job_id = $1
            ",
            &[&self.job_id],
        )
        .await
        .with_context(|| format!("{}: {}", function_name!(), self.job_id))?;

        Ok(())
    }

    pub async fn fail(&self, error: &str) -> Result<(), anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let updated = tx
            .execute(
                "
                update pipeline_engine_job
                set
                    state = 'failed',
                    error = $2,
                    finished_at = now(),
                    updated_at = now()
                where
                    job_id = $1
                    and state = 'running'
                ",
                &[&self.job_id, &error],
            )
            .await
            .with_context(|| format!("{}: job={}", function_name!(), self.job_id))?;

        if updated == 0 {
            tx.rollback().await?;
            warn!(job_id = %self.job_id, "fail called on non-running job, ignoring");
            return Ok(());
        }

        let node_updated = tx
            .execute(
                "
                update pipeline_engine_node
                set
                    state = 'failed',
                    finished_at = now(),
                    updated_at = now()
                where
                    node_id = $1
                    and state = 'in_progress'
                ",
                &[&self.node_id],
            )
            .await
            .with_context(|| format!("{}: node={}", function_name!(), self.node_id))?;

        assert_node_was_in_progress("fail", &self.node_id, node_updated);

        tx.commit().await?;

        Ok(())
    }
}

pub struct TickHandle {
    tasks: Vec<(String, String, tokio::task::JoinHandle<()>)>,
}

impl TickHandle {
    pub async fn join(self) -> Result<(), anyhow::Error> {
        let results = futures::future::join_all(
            self.tasks.into_iter().map(|(node_id, job_id, handle)| async move {
                handle.await.map_err(|e| {
                    error!(node_id = %node_id, job_id = %job_id, "task panic: {e:?}");
                    anyhow::anyhow!("task panic for node={node_id} job={job_id}: {e:?}")
                })
            }),
        )
        .await;

        results.into_iter().collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }
}

impl PipelineEngine {
    async fn assert_invariants(&self) -> Result<(), anyhow::Error> {
        let con = self.db.get().await?;

        let check = |ids: Vec<String>, msg: &str| {
            assert!(ids.is_empty(), "{msg}: {ids:?}");
        };

        let query_ids = |rows: Vec<tokio_postgres::Row>| -> Vec<String> {
            rows.iter().map(|r| r.get::<_, String>(0)).collect()
        };

        check(query_ids(con.query(
            "select n.node_id from pipeline_engine_node n
             where n.state = 'executable'
               and exists (
                 select 1 from pipeline_engine_edge e
                   join pipeline_engine_node pred on e.from_node_id = pred.node_id
                 where e.to_node_id = n.node_id
                   and pred.state not in ('finished', 'cancelled')
               )", &[],
        ).await?), "invariant #1: executable node with unfinished predecessor");

        check(query_ids(con.query(
            "select n.node_id from pipeline_engine_node n
             where n.state = 'in_progress'
               and (select count(*) from pipeline_engine_job j
                    where j.node_id = n.node_id and j.state = 'running') != 1", &[],
        ).await?), "invariant #3: in_progress node without exactly one running job");

        check(query_ids(con.query(
            "select node_id from pipeline_engine_job
             where state = 'running'
             group by node_id having count(*) > 1", &[],
        ).await?), "invariant #7: multiple running jobs for same node");

        check(query_ids(con.query(
            "select j.job_id from pipeline_engine_job j
               join pipeline_engine_node n on j.node_id = n.node_id
             where j.state = 'running' and n.state != 'in_progress'", &[],
        ).await?), "invariant #8: running job on non-in_progress node");

        check(query_ids(con.query(
            "select j.job_id from pipeline_engine_job j
               join pipeline_engine_node n on j.node_id = n.node_id
             where n.state = 'pending' and j.state = 'running'", &[],
        ).await?), "invariant #10: running job for pending node");

        check(query_ids(con.query(
            "select p.pipeline_id from pipeline_engine_pipeline p
             where p.state = 'finished'
               and exists (
                 select 1 from pipeline_engine_node n
                 where n.pipeline_id = p.pipeline_id
                   and n.state not in ('finished', 'cancelled')
               )", &[],
        ).await?), "invariant #11: finished pipeline with non-terminal nodes");

        check(query_ids(con.query(
            "select p.pipeline_id from pipeline_engine_pipeline p
             where p.state = 'cancelled'
               and exists (
                 select 1 from pipeline_engine_node n
                 where n.pipeline_id = p.pipeline_id
                   and n.state in ('executable', 'in_progress')
               )", &[],
        ).await?), "invariant #13: cancelled pipeline with executable/in_progress nodes");

        check(query_ids(con.query(
            "select resource_key from pipeline_engine_node
             where state = 'in_progress' and resource_key is not null
             group by resource_key having count(*) > 1", &[],
        ).await?), "invariant #14: multiple in_progress nodes on same resource");

        check(query_ids(con.query(
            "select e.edge_id from pipeline_engine_edge e
               join pipeline_engine_node n1 on e.from_node_id = n1.node_id
               join pipeline_engine_node n2 on e.to_node_id = n2.node_id
             where n1.pipeline_id != n2.pipeline_id", &[],
        ).await?), "invariant #17: edge crosses pipeline boundary");

        check(query_ids(con.query(
            "select n.node_id from pipeline_engine_node n
             where n.state = 'executable'
               and exists (
                 select 1 from pipeline_engine_edge e
                   join pipeline_engine_node pred on e.from_node_id = pred.node_id
                 where e.to_node_id = n.node_id
                   and pred.state in ('pending', 'executable', 'in_progress', 'failed')
               )", &[],
        ).await?), "invariant #20: executable node with non-terminal predecessor");

        check(query_ids(con.query(
            "select node_id from pipeline_engine_node
             where started_at is not null
               and state in ('pending', 'executable')", &[],
        ).await?), "invariant #21: started_at set on pending/executable node");

        check(query_ids(con.query(
            "select node_id from pipeline_engine_node
             where finished_at is not null
               and state in ('pending', 'executable', 'in_progress')", &[],
        ).await?), "invariant #22: finished_at set on non-terminal node");

        check(query_ids(con.query(
            "select node_id from pipeline_engine_node
             where finished_at is not null
               and started_at is not null
               and finished_at < started_at", &[],
        ).await?), "invariant #23: finished_at < started_at");

        Ok(())
    }
}

fn assert_all_nodes_claimed(expected: usize, actual: u64) {
    assert_eq!(
        actual as usize, expected,
        "claim_nodes: expected to claim {} nodes but only claimed {}. \
         Another tick likely claimed them concurrently.",
        expected, actual
    );
}

fn assert_no_running_job(node_id: &str, running_count: i64) {
    assert_eq!(
        running_count, 0,
        "create_jobs: node {node_id} already has a running job. Double-dispatch detected."
    );
}

fn assert_node_was_in_progress(method: &str, node_id: &str, updated: u64) {
    assert_eq!(
        updated, 1,
        "{method}: node {node_id} was not in_progress (updated {updated} rows). \
         State may have been changed by cancel or another tick."
    );
}

pub struct PipelineEngine {
    db: ConnectionPool,
}

impl PipelineEngine {
    pub async fn new(db: ConnectionPool) -> Result<Self, anyhow::Error> {
        Ok(Self { db })
    }

    pub fn pool(&self) -> ConnectionPool {
        self.db.clone()
    }

    pub async fn create_pipeline(
        &self,
        project_id: &str,
        revision_id: &str,
    ) -> Result<String, anyhow::Error> {
        self.assert_invariants().await?;

        let con = self.db.get().await?;

        let pipeline_id: String = con
            .query_one(
                "
                insert into pipeline_engine_pipeline
                    (project_id, revision_id)
                values
                    ($1, $2)
                returning pipeline_id
                ",
                &[&project_id, &revision_id],
            )
            .await
            .with_context(|| format!("{}: {project_id} {revision_id}", function_name!()))?
            .get("pipeline_id");

        Ok(pipeline_id)
    }

    pub async fn add_node(
        &self,
        pipeline_id: &str,
        node: NodeOptions,
    ) -> Result<String, anyhow::Error> {
        self.assert_invariants().await?;

        let con = self.db.get().await?;

        let resource_key = node.mutates.as_deref();

        let node_id: String = con
            .query_one(
                "
                insert into pipeline_engine_node
                    (pipeline_id, event, resource_key, state)
                values
                    ($1, $2, $3, 'pending')
                returning node_id
                ",
                &[&pipeline_id, &node.event, &resource_key],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), pipeline_id, node.event))?
            .get("node_id");

        Ok(node_id)
    }

    pub async fn add_edge(
        &self,
        from_node_id: &str,
        to_node_id: &str,
    ) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let con = self.db.get().await?;

        con.execute(
            "
            insert into pipeline_engine_edge
                (from_node_id, to_node_id)
            values
                ($1, $2)
            ",
            &[&from_node_id, &to_node_id],
        )
        .await
        .with_context(|| format!("{}: {from_node_id} -> {to_node_id}", function_name!()))?;

        Ok(())
    }

    /// Mark root nodes (those with no incoming edges) as executable.
    /// Call after all nodes and edges have been added.
    pub async fn seal(&self, pipeline_id: &str) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let con = self.db.get().await?;

        con.execute(
            "
            update pipeline_engine_node
            set
                state = 'executable',
                updated_at = now()
            where
                pipeline_id = $1
                and state = 'pending'
                and node_id not in (
                    select e.to_node_id
                    from pipeline_engine_edge e
                        join pipeline_engine_node n
                            on e.to_node_id = n.node_id
                    where n.pipeline_id = $1
                )
            ",
            &[&pipeline_id],
        )
        .await
        .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        Ok(())
    }

    pub async fn nodes(&self, pipeline_id: &str) -> Result<Vec<Node>, anyhow::Error> {
        let con = self.db.get().await?;

        let rows = con
            .query(
                "
                select
                    node_id,
                    event,
                    state,
                    resource_key
                from pipeline_engine_node
                where
                    pipeline_id = $1
                order by created_at
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        let nodes = rows
            .iter()
            .map(|row| Node {
                node_id: row.get("node_id"),
                event: row.get("event"),
                state: row.get("state"),
                resource_key: row.get("resource_key"),
            })
            .collect();

        Ok(nodes)
    }

    pub async fn runnable(&self) -> Result<Vec<Node>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;
        let nodes = self.query_runnable(&tx).await?;
        tx.commit().await?;
        Ok(nodes)
    }

    async fn query_runnable(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<Node>, anyhow::Error> {
        let rows = tx
            .query(
                "
                select
                    n.node_id,
                    n.event,
                    n.state,
                    n.resource_key
                from pipeline_engine_node n
                    join pipeline_engine_pipeline p
                        on n.pipeline_id = p.pipeline_id
                    join revision r
                        on p.revision_id = r.revision_id
                where
                    n.state = 'executable'
                    and p.state = 'active'
                    and not exists (
                        select 1
                        from pipeline_engine_edge e
                            join pipeline_engine_node pred
                                on e.from_node_id = pred.node_id
                        where
                            e.to_node_id = n.node_id
                            and pred.state not in ('finished', 'cancelled')
                    )
                    and (
                        n.resource_key is null
                        or not exists (
                            select 1
                            from pipeline_engine_node older
                                join pipeline_engine_pipeline older_p
                                    on older.pipeline_id = older_p.pipeline_id
                                join revision older_r
                                    on older_p.revision_id = older_r.revision_id
                            where
                                older.resource_key = n.resource_key
                                and older_r.revision_seq < r.revision_seq
                                and older.state not in ('finished', 'cancelled')
                        )
                    )
                for update of n skip locked
                ",
                &[],
            )
            .await
            .with_context(|| format!("{}", function_name!()))?;

        let nodes = rows
            .iter()
            .map(|row| Node {
                node_id: row.get("node_id"),
                event: row.get("event"),
                state: row.get("state"),
                resource_key: row.get("resource_key"),
            })
            .collect();

        Ok(nodes)
    }

    async fn claim_nodes(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
        node_ids: &[&str],
    ) -> Result<(), anyhow::Error> {
        if node_ids.is_empty() {
            return Ok(());
        }

        let claimed = tx
            .execute(
                "
                update pipeline_engine_node
                set
                    state = 'in_progress',
                    started_at = now(),
                    updated_at = now()
                where
                    node_id = any($1)
                    and state = 'executable'
                ",
                &[&node_ids],
            )
            .await
            .with_context(|| format!("{}", function_name!()))?;

        assert_all_nodes_claimed(node_ids.len(), claimed);

        Ok(())
    }

    async fn create_jobs(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
        node_ids: &[&str],
    ) -> Result<Vec<String>, anyhow::Error> {
        let mut job_ids = vec![];

        for node_id in node_ids {
            let existing_running: i64 = tx
                .query_one(
                    "
                    select count(*) as c
                    from pipeline_engine_job
                    where
                        node_id = $1
                        and state = 'running'
                    ",
                    &[node_id],
                )
                .await
                .with_context(|| format!("{}: check existing {node_id}", function_name!()))?
                .get("c");

            assert_no_running_job(node_id, existing_running);

            let job_id: String = tx
                .query_one(
                    "
                    insert into pipeline_engine_job
                        (node_id)
                    values
                        ($1)
                    returning job_id
                    ",
                    &[node_id],
                )
                .await
                .with_context(|| format!("{}: {node_id}", function_name!()))?
                .get("job_id");

            job_ids.push(job_id);
        }

        Ok(job_ids)
    }

    fn spawn_job<F, Fut>(
        &self,
        node: Node,
        job_id: String,
        dispatch: F,
    ) -> (String, String, tokio::task::JoinHandle<()>)
    where
        F: Fn(Node) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
    {
        let node_id = node.node_id.clone();
        let event = node.event.clone();

        let handle = JobHandle {
            db: self.db.clone(),
            job_id: job_id.clone(),
            node_id: node_id.clone(),
        };

        let span = info_span!("pipeline_job", job_id = %job_id, node_id = %node_id, event = %event);

        let heartbeat_handle = handle.clone();
        let work_span = span.clone();
        let join_handle = tokio::spawn(
            async move {
                info!("job started");

                let heartbeat_task = tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        if let Err(e) = heartbeat_handle.heartbeat().await {
                            error!("heartbeat failed: {e:?}");
                        }
                    }
                });

                let work_task = tokio::spawn(dispatch(node).instrument(work_span));

                let result = work_task.await;
                heartbeat_task.abort();

                match result {
                    Ok(Ok(())) => {
                        info!("job succeeded");
                        if let Err(e) = handle.succeed().await {
                            error!("failed to record success: {e:?}");
                        }
                    }
                    Ok(Err(e)) => {
                        let err_msg = format!("{e:?}");
                        error!("job failed: {err_msg}");
                        if let Err(e) = handle.fail(&err_msg).await {
                            error!("failed to record failure: {e:?}");
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("task panic: {e:?}");
                        error!("{err_msg}");
                        if let Err(e) = handle.fail(&err_msg).await {
                            error!("failed to record panic: {e:?}");
                        }
                    }
                }
            }
            .instrument(span),
        );

        (node_id, job_id, join_handle)
    }


    /// Atomically finds runnable nodes, claims them, creates jobs, and executes
    /// work concurrently. Blocks until all dispatched work completes.
    ///
    /// On dispatch success: node → finished, successors promoted.
    /// On dispatch failure: node → failed (requires manual retry or pipeline cancel).
    /// On task panic: returns Err (surfaces as 5xx in the API).
    /// On stale heartbeat: node → failed (same as dispatch failure).
    async fn release_stale_nodes(&self) -> Result<(), anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let released = tx
            .execute(
                "
                update pipeline_engine_node
                set
                    state = 'executable',
                    started_at = null,
                    updated_at = now()
                where
                    state = 'in_progress'
                    and node_id in (
                        select node_id
                        from pipeline_engine_job
                        where
                            state = 'running'
                            and last_heartbeat_at < now() - interval '60 seconds'
                    )
                ",
                &[],
            )
            .await
            .with_context(|| format!("{}: release nodes", function_name!()))?;

        tx.execute(
            "
            update pipeline_engine_job
            set
                state = 'failed',
                error = 'stale heartbeat',
                finished_at = now(),
                updated_at = now()
            where
                state = 'running'
                and last_heartbeat_at < now() - interval '60 seconds'
            ",
            &[],
        )
        .await
        .with_context(|| format!("{}: mark jobs failed", function_name!()))?;

        tx.commit().await?;

        if released > 0 {
            warn!("released {released} nodes with stale heartbeats");
        }

        Ok(())
    }

    /// Finds runnable nodes, claims them, creates jobs, and spawns tasks.
    ///
    /// Returns a `TickHandle` representing the spawned work. Callers that need
    /// to wait for completion (tests, graceful shutdown) call `handle.join()`.
    /// Fire-and-forget callers (the production cron loop) drop the handle.
    ///
    /// Nodes are `in_progress` in the DB immediately when tick() returns —
    /// subsequent ticks will not re-dispatch them.
    pub async fn tick<F, Fut>(&self, dispatch: F) -> Result<TickHandle, anyhow::Error>
    where
        F: Fn(Node) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
    {
        self.assert_invariants().await?;
        self.release_stale_nodes().await?;
        self.promote_unblocked_nodes().await?;
        self.assert_invariants().await?;

        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let runnable = self.query_runnable(&tx).await?;
        let node_ids: Vec<&str> = runnable.iter().map(|n| n.node_id.as_str()).collect();
        self.claim_nodes(&tx, &node_ids).await?;
        let job_ids = self.create_jobs(&tx, &node_ids).await?;

        tx.commit().await?;

        let tasks: Vec<_> = runnable
            .into_iter()
            .zip(job_ids)
            .map(|(node, job_id)| self.spawn_job(node, job_id, dispatch.clone()))
            .collect();

        Ok(TickHandle { tasks })
    }

    async fn promote_unblocked_nodes(&self) -> Result<(), anyhow::Error> {
        let con = self.db.get().await?;

        con.execute(
            "
            update pipeline_engine_node
            set
                state = 'executable',
                updated_at = now()
            where
                state = 'pending'
                and not exists (
                    select 1
                    from pipeline_engine_edge e
                        join pipeline_engine_node pred
                            on e.from_node_id = pred.node_id
                    where
                        e.to_node_id = pipeline_engine_node.node_id
                        and pred.state not in ('finished', 'cancelled')
                )
            ",
            &[],
        )
        .await
        .with_context(|| format!("{}", function_name!()))?;

        Ok(())
    }

    pub async fn why_blocked(&self, node_id: &str) -> Result<Option<BlockReason>, anyhow::Error> {
        let con = self.db.get().await?;

        let row = con
            .query_one(
                "
                select node_id, event, state, resource_key
                from pipeline_engine_node
                where node_id = $1
                ",
                &[&node_id],
            )
            .await
            .with_context(|| format!("{}: {node_id}", function_name!()))?;

        let state: String = row.get("state");

        if state != "executable" {
            let pending_preds: Vec<Node> = con
                .query(
                    "
                    select
                        pred.node_id,
                        pred.event,
                        pred.state,
                        pred.resource_key
                    from pipeline_engine_edge e
                        join pipeline_engine_node pred
                            on e.from_node_id = pred.node_id
                    where
                        e.to_node_id = $1
                        and pred.state not in ('finished', 'cancelled')
                    ",
                    &[&node_id],
                )
                .await
                .with_context(|| format!("{}: predecessors {node_id}", function_name!()))?
                .iter()
                .map(|r| Node {
                    node_id: r.get("node_id"),
                    event: r.get("event"),
                    state: r.get("state"),
                    resource_key: r.get("resource_key"),
                })
                .collect();

            return Ok(Some(BlockReason::NotExecutable {
                state,
                pending_predecessors: pending_preds,
            }));
        }

        let resource_key: Option<String> = row.get("resource_key");

        if let Some(resource_key) = resource_key {
            let blocker = con
                .query_opt(
                    "
                    select
                        older.node_id,
                        older.event,
                        older.state,
                        older.resource_key,
                        older_p.revision_id
                    from pipeline_engine_node older
                        join pipeline_engine_pipeline older_p
                            on older.pipeline_id = older_p.pipeline_id
                        join revision older_r
                            on older_p.revision_id = older_r.revision_id
                        join pipeline_engine_node me
                            on me.node_id = $1
                        join pipeline_engine_pipeline my_p
                            on me.pipeline_id = my_p.pipeline_id
                        join revision my_r
                            on my_p.revision_id = my_r.revision_id
                    where
                        older.resource_key = $2
                        and older_r.revision_seq < my_r.revision_seq
                        and older.state not in ('finished', 'cancelled')
                    limit 1
                    ",
                    &[&node_id, &resource_key],
                )
                .await
                .with_context(|| format!("{}: resource {node_id}", function_name!()))?;

            if let Some(blocker_row) = blocker {
                return Ok(Some(BlockReason::ResourceContention {
                    blocking_node: Node {
                        node_id: blocker_row.get("node_id"),
                        event: blocker_row.get("event"),
                        state: blocker_row.get("state"),
                        resource_key: blocker_row.get("resource_key"),
                    },
                    blocking_revision: blocker_row.get("revision_id"),
                }));
            }
        }

        Ok(None)
    }

    pub async fn edges(&self, pipeline_id: &str) -> Result<Vec<Edge>, anyhow::Error> {
        let con = self.db.get().await?;

        let rows = con
            .query(
                "
                select
                    e.edge_id,
                    e.from_node_id,
                    e.to_node_id
                from pipeline_engine_edge e
                    join pipeline_engine_node n
                        on e.from_node_id = n.node_id
                where
                    n.pipeline_id = $1
                order by e.created_at
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        let edges = rows
            .iter()
            .map(|row| Edge {
                edge_id: row.get("edge_id"),
                from_node_id: row.get("from_node_id"),
                to_node_id: row.get("to_node_id"),
            })
            .collect();

        Ok(edges)
    }

    pub async fn active_pipelines(&self, project_id: &str) -> Result<Vec<Pipeline>, anyhow::Error> {
        let con = self.db.get().await?;

        let rows = con
            .query(
                "
                select
                    pipeline_id,
                    project_id,
                    revision_id,
                    state
                from pipeline_engine_pipeline
                where
                    project_id = $1
                    and state = 'active'
                order by created_at
                ",
                &[&project_id],
            )
            .await
            .with_context(|| format!("{}: {project_id}", function_name!()))?;

        let pipelines = rows
            .iter()
            .map(|row| Pipeline {
                pipeline_id: row.get("pipeline_id"),
                project_id: row.get("project_id"),
                revision_id: row.get("revision_id"),
                state: row.get("state"),
            })
            .collect();

        Ok(pipelines)
    }

    pub async fn latest_finished_pipeline(&self, project_id: &str) -> Result<Option<Pipeline>, anyhow::Error> {
        let con = self.db.get().await?;

        let row = con
            .query_opt(
                "
                select
                    p.pipeline_id,
                    p.project_id,
                    p.revision_id,
                    p.state
                from pipeline_engine_pipeline p
                    join revision r
                        on p.revision_id = r.revision_id
                where
                    p.project_id = $1
                    and p.state = 'finished'
                order by r.revision_seq desc
                limit 1
                ",
                &[&project_id],
            )
            .await
            .with_context(|| format!("{}: {project_id}", function_name!()))?;

        Ok(row.map(|r| Pipeline {
            pipeline_id: r.get("pipeline_id"),
            project_id: r.get("project_id"),
            revision_id: r.get("revision_id"),
            state: r.get("state"),
        }))
    }

    pub async fn node_job(&self, node_id: &str) -> Result<Option<Job>, anyhow::Error> {
        let con = self.db.get().await?;

        let row = con
            .query_opt(
                "
                select
                    job_id,
                    node_id,
                    state,
                    error
                from pipeline_engine_job
                where
                    node_id = $1
                order by created_at desc
                limit 1
                ",
                &[&node_id],
            )
            .await
            .with_context(|| format!("{}: {node_id}", function_name!()))?;

        Ok(row.map(|r| Job {
            job_id: r.get("job_id"),
            node_id: r.get("node_id"),
            state: r.get("state"),
            error: r.get("error"),
        }))
    }

    pub async fn retry(&self, node_id: &str) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let con = self.db.get().await?;

        let updated = con
            .execute(
                "
                update pipeline_engine_node
                set
                    state = 'executable',
                    started_at = null,
                    finished_at = null,
                    updated_at = now()
                where
                    node_id = $1
                    and state = 'failed'
                ",
                &[&node_id],
            )
            .await
            .with_context(|| format!("{}: {node_id}", function_name!()))?;

        if updated == 0 {
            anyhow::bail!("node {node_id} is not in failed state");
        }

        self.assert_invariants().await?;
        Ok(())
    }

    pub async fn cancel(&self, pipeline_id: &str) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        tx.execute(
            "
            update pipeline_engine_node
            set
                state = 'cancelled',
                updated_at = now()
            where
                pipeline_id = $1
                and state != 'finished'
            ",
            &[&pipeline_id],
        )
        .await
        .with_context(|| format!("{}: cancel nodes {pipeline_id}", function_name!()))?;

        tx.execute(
            "
            update pipeline_engine_pipeline
            set
                state = 'cancelled',
                finished_at = now(),
                updated_at = now()
            where
                pipeline_id = $1
            ",
            &[&pipeline_id],
        )
        .await
        .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        tx.commit().await?;

        self.assert_invariants().await?;
        Ok(())
    }
}
