use ant_library::db::ConnectionPool;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use stdext::function_name;
use tracing::{error, info, info_span, warn, Instrument};

use super::node::NodeSpec;

/// Whether the engine is asking the handler to deploy forward or unwind a previous deployment.
///
/// Handlers should `match` on this to determine what action to take. For `Deploy`, apply
/// the desired state for `revision_id`. For `Unwind`, restore the resource to the state
/// described by `restore_revision_id` (or tear down if `None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchDirection {
    /// Apply this revision's desired state to the resource.
    Deploy,
    /// Reverse a previous deployment. The resource should be restored to the state it was
    /// in before this revision touched it.
    Unwind {
        /// The revision that was last successfully deployed to this resource before the
        /// current one. The handler should apply THIS revision's state to the resource.
        ///
        /// `None` means nothing was deployed to this resource before the current revision
        /// (new resource added by this revision, or node has no resource_key). The handler
        /// should tear down / remove whatever the current revision created.
        restore_revision_id: Option<String>,
    },
}

/// The payload given to the dispatch handler on each `tick()`. Contains everything the
/// handler needs to perform work: which direction (deploy or unwind), which revision,
/// and the node's event data describing the resource coordinates.
///
/// The engine constructs this and passes it to the closure provided to `tick()`. The
/// handler returns `Ok(())` on success or `Err(...)` on failure — the engine handles
/// state transitions based on the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    /// Whether this is a forward deployment or an unwind of a previous deployment.
    pub direction: DispatchDirection,
    /// The revision this node belongs to (from the pipeline).
    pub revision_id: String,
    /// The node being dispatched. Contains `event` (JSON with resource coordinates)
    /// and `resource_key` (the FIFO scheduling key for this resource).
    pub node: Node,
}

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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NodeEvent {
    pub node_id: String,
    pub to_state: String,
    pub reason: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub node_id: String,
    pub revision_id: String,
    pub event: String,
    pub state: String,
    pub resource_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        blocking_pipeline: String,
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

        let pipeline_state: String = tx
            .query_one(
                "
                select p.state
                from pipeline_engine_node n
                  join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                where n.node_id = $1
                ",
                &[&self.node_id],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: lookup pipeline state node={}",
                    function_name!(),
                    self.node_id
                )
            })?
            .get("state");

        let is_unwinding = pipeline_state == "unwinding";
        let target_state = if is_unwinding { "unwound" } else { "finished" };

        let node_updated = tx
            .execute(
                &format!(
                    "
                    update pipeline_engine_node
                    set
                        state = '{target_state}',
                        finished_at = now(),
                        updated_at = now()
                    where
                        node_id = $1
                        and state = 'in_progress'
                    "
                ),
                &[&self.node_id],
            )
            .await
            .with_context(|| format!("{}: node={}", function_name!(), self.node_id))?;

        assert_node_was_in_progress("succeed", &self.node_id, node_updated);

        log_node_transition(
            &tx,
            &self.node_id,
            target_state,
            &format!("succeed job={}", self.job_id),
        )
        .await?;

        if !is_unwinding {
            // Promote successors whose predecessors are all done
            let promoted: Vec<String> = tx
                .query(
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
                    returning node_id
                    ",
                    &[&self.node_id],
                )
                .await
                .with_context(|| format!("{}: promote node={}", function_name!(), self.node_id))?
                .iter()
                .map(|r| r.get("node_id"))
                .collect();

            let promoted_refs: Vec<&str> = promoted.iter().map(|s| s.as_str()).collect();
            log_node_transitions_bulk(&tx, &promoted_refs, "executable", "promote").await?;
        }

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

        if !is_unwinding {
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
                .with_context(|| {
                    format!("{}: finalize pipeline={}", function_name!(), pipeline_id)
                })?;

                log_pipeline_transition(&tx, &pipeline_id, "finished", "all_nodes_terminal")
                    .await?;
            }
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

        let pipeline_state: String = tx
            .query_one(
                "
                select p.state
                from pipeline_engine_node n
                  join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                where n.node_id = $1
                ",
                &[&self.node_id],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: lookup pipeline state node={}",
                    function_name!(),
                    self.node_id
                )
            })?
            .get("state");

        let target_state = if pipeline_state == "unwinding" {
            "unwind_failed"
        } else {
            "failed"
        };

        let node_updated = tx
            .execute(
                &format!(
                    "
                    update pipeline_engine_node
                    set
                        state = '{target_state}',
                        finished_at = now(),
                        updated_at = now()
                    where
                        node_id = $1
                        and state = 'in_progress'
                    "
                ),
                &[&self.node_id],
            )
            .await
            .with_context(|| format!("{}: node={}", function_name!(), self.node_id))?;

        assert_node_was_in_progress("fail", &self.node_id, node_updated);

        log_node_transition(
            &tx,
            &self.node_id,
            target_state,
            &format!("fail job={}", self.job_id),
        )
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

pub struct TickHandle {
    tasks: Vec<(String, String, tokio::task::JoinHandle<()>)>,
}

impl TickHandle {
    pub async fn join(self) -> Result<(), anyhow::Error> {
        let results = futures::future::join_all(self.tasks.into_iter().map(
            |(node_id, job_id, handle)| async move {
                handle.await.map_err(|e| {
                    error!(node_id = %node_id, job_id = %job_id, "task panic: {e:?}");
                    anyhow::anyhow!("task panic for node={node_id} job={job_id}: {e:?}")
                })
            },
        ))
        .await;

        results.into_iter().collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }
}

impl PipelineEngine {
    async fn assert_invariants(&self) -> Result<(), anyhow::Error> {
        let con = self.db.get().await?;

        macro_rules! check_invariant {
            ($query:expr, $msg:expr) => {{
                let rows = con.query($query, &[]).await.context($msg)?;
                if !rows.is_empty() {
                    let columns: Vec<&str> = rows[0].columns().iter().map(|c| c.name()).collect();
                    let violations: Vec<Vec<String>> = rows
                        .iter()
                        .map(|r| {
                            (0..r.len())
                                .map(|i| {
                                    r.try_get::<_, String>(i).unwrap_or_else(|_| {
                                        format!("{:?}", r.try_get::<_, i64>(i).unwrap_or(-1))
                                    })
                                })
                                .collect()
                        })
                        .collect();
                    panic!(
                        "{}\n  columns: {:?}\n  rows: {:?}\n  query: {}",
                        $msg, columns, violations, $query
                    );
                }
            }};
        }

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.state = 'executable' and
                p.state in ('active', 'unwinding') and
                exists (
                    select 1 from pipeline_engine_edge e
                        join pipeline_engine_node pred on e.from_node_id = pred.node_id
                    where
                        e.to_node_id = n.node_id and
                        pred.state not in ('finished', 'cancelled')
                )
            ",
            "invariant #1: executable node with unfinished predecessor"
        );

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.state = 'in_progress' and
                p.state in ('active', 'unwinding') and
                (
                    select count(*)
                    from pipeline_engine_job j
                    where
                        j.node_id = n.node_id and j.state = 'running'
                ) != 1
            ",
            "invariant #3: in_progress node without exactly one running job"
        );

        check_invariant!(
            "
            select j.node_id
            from pipeline_engine_job j
                join pipeline_engine_node n on n.node_id = j.node_id
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                j.state = 'running' and
                p.state in ('active', 'unwinding')
            group by j.node_id having count(*) > 1
            ",
            "invariant #7: multiple running jobs for same node"
        );

        check_invariant!(
            "
            select j.job_id
            from pipeline_engine_job j
                join pipeline_engine_node n on j.node_id = n.node_id
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                j.state = 'running' and
                n.state != 'in_progress' and
                p.state in ('active', 'unwinding')
            ",
            "invariant #8: running job on non-in_progress node"
        );

        check_invariant!(
            "
            select j.job_id
            from pipeline_engine_job j
               join pipeline_engine_node n on j.node_id = n.node_id
               join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.state = 'pending' and
                j.state = 'running' and
                p.state in ('active', 'unwinding')
            ",
            "invariant #10: running job for pending node"
        );

        check_invariant!(
            "
            select p.pipeline_id
            from pipeline_engine_pipeline p
            where
                p.state in ('finished', 'unwound') and
                exists (
                    select 1 from pipeline_engine_node n
                    where
                        n.pipeline_id = p.pipeline_id and
                        n.state not in ('finished', 'cancelled', 'unwound', 'unwind_failed')
                )
            ",
            "invariant #11: terminal pipeline with non-terminal nodes"
        );

        // -- Once we cancel a pipeline for good, we ignore it from invariants
        //
        // check_invariant!(
        //     "
        //     select p.pipeline_id
        //     from pipeline_engine_pipeline p
        //     where p.state = 'cancelled'
        //        and exists (
        //          select 1 from pipeline_engine_node n
        //          where n.pipeline_id = p.pipeline_id
        //            and n.state in ('executable', 'in_progress')
        //        )",
        //     "invariant #13: cancelled pipeline with executable/in_progress nodes"
        // );

        check_invariant!(
            "
            select n.resource_key
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.state = 'in_progress' and
                n.resource_key is not null and
                p.state in ('active', 'unwinding')
            group by n.resource_key having count(*) > 1
            ",
            "invariant #14: multiple in_progress nodes on same resource"
        );

        check_invariant!(
            "
            select e.edge_id
            from pipeline_engine_edge e
                join pipeline_engine_node n1 on e.from_node_id = n1.node_id
                join pipeline_engine_node n2 on e.to_node_id = n2.node_id
                join pipeline_engine_pipeline p on p.pipeline_id = n1.pipeline_id
            where
                n1.pipeline_id != n2.pipeline_id and
                p.state in ('active', 'unwinding')
            ",
            "invariant #17: edge crosses pipeline boundary"
        );

        check_invariant!(
            "select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.state = 'executable' and
                p.state in ('active', 'unwinding') and
                exists (
                    select 1 from pipeline_engine_edge e
                        join pipeline_engine_node pred on e.from_node_id = pred.node_id
                    where
                        e.to_node_id = n.node_id and
                        pred.state in ('pending', 'executable', 'in_progress', 'failed')
                )
            ",
            "invariant #20: executable node with non-terminal predecessor"
        );

        check_invariant!(
            "select node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.started_at is not null and
                n.state in ('pending', 'executable') and
                p.state in ('active', 'unwinding')
            ",
            "invariant #21: started_at set on pending/executable node"
        );

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.finished_at is not null and
                n.state in ('pending', 'executable', 'in_progress') and
                p.state in ('active', 'unwinding')
            ",
            "invariant #22: finished_at set on non-terminal node"
        );

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on p.pipeline_id = n.pipeline_id
            where
                n.finished_at is not null and
                n.started_at is not null and
                n.finished_at < started_at and
                p.state in ('active', 'unwinding')
            ",
            "invariant #23: finished_at < started_at"
        );

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
            where
                p.state = 'unwinding' and
                n.state = 'executable'
            ",
            "invariant #25: executable node in unwinding pipeline"
        );

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
            where
                n.state in ('unwound', 'unwind_failed') and
                p.state not in ('unwinding', 'unwound', 'cancelled')
            ",
            "invariant #26: unwound/unwind_failed node in non-unwinding pipeline"
        );

        check_invariant!(
            "
            select n.node_id
            from pipeline_engine_node n
                join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
            where
                p.state = 'unwinding' and
                n.state = 'unwound' and
                exists (
                    select 1 from pipeline_engine_edge e
                        join pipeline_engine_node successor on e.to_node_id = successor.node_id
                    where
                        e.from_node_id = n.node_id and
                        successor.pipeline_id = n.pipeline_id and
                        successor.is_unwind_boundary = false and
                        successor.state in ('finished', 'failed')
                )
            ",
            "invariant #27: unwound node with non-unwound successor in scope (reverse order violated)");

        check_invariant!(
            "
            with ordered_events as (
                select
                    e.node_id,
                    e.to_state,
                    lag(e.to_state) over (partition by e.node_id order by e.event_seq) as from_state
                from pipeline_engine_node_event e
                    join pipeline_engine_node n on e.node_id = n.node_id
                    join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                where
                    p.state in ('active', 'unwinding')
            )
            select node_id from ordered_events
            where from_state is not null
              and (from_state, to_state) not in (
                ('pending', 'executable'),
                ('pending', 'cancelled'),
                ('executable', 'in_progress'),
                ('executable', 'cancelled'),
                ('in_progress', 'finished'),
                ('in_progress', 'failed'),
                ('in_progress', 'cancelled'),
                ('in_progress', 'executable'),
                ('in_progress', 'unwound'),
                ('in_progress', 'unwind_failed'),
                ('failed', 'executable'),
                ('failed', 'cancelled'),
                ('failed', 'in_progress'),
                ('finished', 'in_progress'),
                ('finished', 'cancelled'),
                ('unwind_failed', 'finished'),
                ('unwind_failed', 'cancelled')
              )",
            "invariant #30: illegal state transition in event log"
        );

        check_invariant!(
            "
            select p.pipeline_id
            from pipeline_engine_pipeline p
            where
                p.state = 'unwinding' and
                not exists (
                    select 1 from pipeline_engine_node n
                    where
                        n.pipeline_id = p.pipeline_id
                        and n.unwind_on_failure = true
                        and n.state in ('failed', 'unwound', 'unwind_failed', 'in_progress')
                )
            ",
            "invariant #31: unwinding pipeline with no unwind_on_failure node that triggered it"
        );

        Ok(())
    }
}

async fn log_node_transition(
    tx: &tokio_postgres::Transaction<'_>,
    node_id: &str,
    to_state: &str,
    reason: &str,
) -> Result<(), anyhow::Error> {
    tx.execute(
        "
        insert into pipeline_engine_node_event
            (node_id, to_state, reason)
        values
            ($1, $2, $3)
        ",
        &[&node_id, &to_state, &reason],
    )
    .await
    .with_context(|| format!("log_node_transition: node={node_id} ->{to_state} reason={reason}"))?;
    Ok(())
}

async fn log_node_transitions_bulk(
    tx: &tokio_postgres::Transaction<'_>,
    node_ids: &[&str],
    to_state: &str,
    reason: &str,
) -> Result<(), anyhow::Error> {
    if node_ids.is_empty() {
        return Ok(());
    }
    tx.execute(
        "
        insert into pipeline_engine_node_event
            (node_id, to_state, reason)
        select unnest($1::text[]), $2, $3
        ",
        &[&node_ids, &to_state, &reason],
    )
    .await
    .with_context(|| format!("log_node_transitions_bulk: ->{to_state} reason={reason}"))?;
    Ok(())
}

async fn log_pipeline_transition(
    tx: &tokio_postgres::Transaction<'_>,
    pipeline_id: &str,
    to_state: &str,
    reason: &str,
) -> Result<(), anyhow::Error> {
    tx.execute(
        "
        insert into pipeline_engine_pipeline_event
            (pipeline_id, to_state, reason)
        values
            ($1, $2, $3)
        ",
        &[&pipeline_id, &to_state, &reason],
    )
    .await
    .with_context(|| {
        format!("log_pipeline_transition: pipeline={pipeline_id} ->{to_state} reason={reason}")
    })?;
    Ok(())
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
    /// Create a new engine backed by the given connection pool. Cheap — no state beyond the pool.
    pub async fn new(db: ConnectionPool) -> Result<Self, anyhow::Error> {
        Ok(Self { db })
    }

    /// Returns the underlying connection pool (for test setup and raw queries).
    pub fn pool(&self) -> ConnectionPool {
        self.db.clone()
    }

    /// Create a new pipeline for the given project and revision. Returns the pipeline ID.
    /// Pipeline starts in `active` state with no nodes. Must call `seal()` after adding
    /// all nodes and edges before the pipeline participates in `tick()`.
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

    /// Add a node to a pipeline. Returns the node ID. Nodes start in `pending` state
    /// and won't be dispatched until `seal()` + all predecessors complete.
    pub async fn add_node(
        &self,
        pipeline_id: &str,
        node: NodeSpec,
    ) -> Result<String, anyhow::Error> {
        self.assert_invariants().await?;

        let con = self.db.get().await?;

        let resource_key = node.mutates.as_deref();

        let node_id: String = con
            .query_one(
                "
                insert into pipeline_engine_node
                    (pipeline_id, event, resource_key, is_unwind_boundary, unwind_on_failure, state)
                values
                    ($1, $2, $3, $4, $5, 'pending')
                returning node_id
                ",
                &[
                    &pipeline_id,
                    &node.event,
                    &resource_key,
                    &node.options.is_unwind_boundary,
                    &node.options.unwind_on_failure,
                ],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), pipeline_id, node.event))?
            .get("node_id");

        Ok(node_id)
    }

    /// Add a directed edge: `from_node_id` must complete before `to_node_id` can run.
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

    /// Finalize the DAG: promotes root nodes (no incoming edges) to `executable`.
    /// Call exactly once after all nodes and edges are added. After seal, the pipeline
    /// participates in `tick()` dispatch. Cannot add nodes or edges after sealing.
    pub async fn seal(&self, pipeline_id: &str) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let sealed: Vec<String> = tx
            .query(
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
                returning node_id
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {pipeline_id}", function_name!()))?
            .iter()
            .map(|r| r.get("node_id"))
            .collect();

        let sealed_refs: Vec<&str> = sealed.iter().map(|s| s.as_str()).collect();
        log_node_transitions_bulk(&tx, &sealed_refs, "executable", "seal").await?;

        tx.commit().await?;

        Ok(())
    }

    /// List all nodes in a pipeline, ordered by creation time.
    pub async fn nodes(&self, pipeline_id: &str) -> Result<Vec<Node>, anyhow::Error> {
        let con = self.db.get().await?;

        let rows = con
            .query(
                "
                select
                    n.node_id,
                    p.revision_id,
                    n.event,
                    n.state,
                    n.resource_key
                from pipeline_engine_node n
                    join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                where
                    n.pipeline_id = $1
                order by n.created_at
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        let nodes = rows
            .iter()
            .map(|row| Node {
                node_id: row.get("node_id"),
                revision_id: row.get("revision_id"),
                event: row.get("event"),
                state: row.get("state"),
                resource_key: row.get("resource_key"),
            })
            .collect();

        Ok(nodes)
    }

    /// List nodes that are currently runnable (executable, predecessors done, no FIFO block).
    /// Does not claim them — use `tick()` for that. Useful for diagnostics/UI.
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
                    p.revision_id,
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
                                and older.state not in ('finished', 'cancelled', 'unwound', 'unwind_failed')
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
                revision_id: row.get("revision_id"),
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

        log_node_transitions_bulk(tx, node_ids, "in_progress", "claim").await?;

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
        dispatch_arg: Dispatch,
        job_id: String,
        dispatch: F,
    ) -> (String, String, tokio::task::JoinHandle<()>)
    where
        F: Fn(Dispatch) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
    {
        let node_id = dispatch_arg.node.node_id.clone();
        let event = dispatch_arg.node.event.clone();

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

                let work_task = tokio::spawn(dispatch(dispatch_arg).instrument(work_span));

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
    #[tracing::instrument(skip_all)]
    async fn release_stale_nodes(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<(), anyhow::Error> {
        let released_nodes: Vec<String> = tx
            .query(
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
                returning node_id
                ",
                &[],
            )
            .await
            .with_context(|| format!("{}: release nodes", function_name!()))?
            .iter()
            .map(|r| r.get("node_id"))
            .collect();

        let released_refs: Vec<&str> = released_nodes.iter().map(|s| s.as_str()).collect();
        log_node_transitions_bulk(&tx, &released_refs, "executable", "release_stale").await?;

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

        if !released_nodes.is_empty() {
            warn!(
                "released {} nodes with stale heartbeats",
                released_nodes.len()
            );
        }

        Ok(())
    }

    /// The main work loop. Call repeatedly (e.g. on a cron/timer).
    ///
    /// Each tick:
    /// 1. Releases nodes with stale heartbeats (crashed workers)
    /// 2. Promotes pending nodes whose predecessors are all done
    /// 3. Detects failed nodes with `unwind_on_failure` and transitions their pipeline
    ///    to `unwinding` (cancelling unreached nodes via forward cascade)
    /// 4. Finds forward-runnable nodes (active pipelines) and unwind-eligible nodes
    ///    (unwinding pipelines, reverse-DAG order)
    /// 5. Claims them all atomically (`in_progress`), spawns dispatch tasks
    ///
    /// Returns a `TickHandle`. Call `handle.join()` to await completion (tests,
    /// graceful shutdown). Drop the handle for fire-and-forget (production cron).
    ///
    /// Dispatch success → node becomes `finished` (forward) or `unwound` (unwind).
    /// Dispatch failure → node becomes `failed` (forward) or `unwind_failed` (unwind).
    #[tracing::instrument(skip_all)]
    pub async fn tick<F, Fut>(&self, dispatch: F) -> Result<TickHandle, anyhow::Error>
    where
        F: Fn(Dispatch) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
    {
        self.assert_invariants().await?;

        let (runnable, unwind_eligible, job_ids) = {
            let mut con = self.db.get().await?;
            let tx = con.transaction().await?;

            self.release_stale_nodes(&tx).await?;
            self.promote_unblocked_nodes(&tx).await?;
            self.detect_and_trigger_unwind(&tx).await?;
            self.complete_unwinding_pipelines(&tx).await?;

            let runnable = self.query_runnable(&tx).await?;
            let unwind_eligible = self.query_unwind_eligible(&tx).await?;

            let forward_ids: Vec<&str> = runnable.iter().map(|n| n.node_id.as_str()).collect();
            let unwind_ids: Vec<&str> =
                unwind_eligible.iter().map(|n| n.node_id.as_str()).collect();

            self.claim_nodes(&tx, &forward_ids).await?;
            self.claim_unwind_nodes(&tx, &unwind_ids).await?;

            let all_node_ids: Vec<&str> = forward_ids
                .iter()
                .chain(unwind_ids.iter())
                .copied()
                .collect();
            let job_ids = self.create_jobs(&tx, &all_node_ids).await?;

            tx.commit().await?;
            (runnable, unwind_eligible, job_ids)
        };

        self.assert_invariants().await?;

        let forward_dispatches: Vec<Dispatch> = runnable
            .into_iter()
            .map(|node| Dispatch {
                direction: DispatchDirection::Deploy,
                revision_id: node.revision_id.clone(),
                node,
            })
            .collect();

        let unwind_dispatches = self.build_unwind_dispatches(unwind_eligible).await?;

        let all_dispatches: Vec<Dispatch> = forward_dispatches
            .into_iter()
            .chain(unwind_dispatches)
            .collect();

        let tasks: Vec<_> = all_dispatches
            .into_iter()
            .zip(job_ids)
            .map(|(d, job_id)| self.spawn_job(d, job_id, dispatch.clone()))
            .collect();

        Ok(TickHandle { tasks })
    }

    #[tracing::instrument(skip_all)]
    async fn promote_unblocked_nodes(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<(), anyhow::Error> {
        let promoted: Vec<String> = tx
            .query(
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
                returning node_id
                ",
                &[],
            )
            .await
            .with_context(|| format!("{}", function_name!()))?
            .iter()
            .map(|r| r.get("node_id"))
            .collect();

        let promoted_refs: Vec<&str> = promoted.iter().map(|s| s.as_str()).collect();
        log_node_transitions_bulk(tx, &promoted_refs, "executable", "promote").await?;

        Ok(())
    }

    /// Diagnose why a node isn't running. Returns `None` if the node is runnable.
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
                        p.revision_id,
                        pred.event,
                        pred.state,
                        pred.resource_key
                    from pipeline_engine_edge e
                        join pipeline_engine_node pred
                            on e.from_node_id = pred.node_id
                        join pipeline_engine_pipeline p
                            on pred.pipeline_id = p.pipeline_id
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
                    revision_id: r.get("revision_id"),
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
                        older_p.revision_id,
                        older_p.pipeline_id
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
                        and older.state not in ('finished', 'cancelled', 'unwound', 'unwind_failed')
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
                        revision_id: blocker_row.get("revision_id"),
                        event: blocker_row.get("event"),
                        state: blocker_row.get("state"),
                        resource_key: blocker_row.get("resource_key"),
                    },
                    blocking_pipeline: blocker_row.get("pipeline_id"),
                    blocking_revision: blocker_row.get("revision_id"),
                }));
            }
        }

        Ok(None)
    }

    // /// Completely, irreversibly, abort a pipeline for good. Releases locks for resources and stops the propagation.
    // pub async fn abort_pipeline(&self, pipeline_id: &str) -> Result<(), anyhow::Error> {
    //     //
    // }

    /// Return nodes grouped by topological layer (BFS from roots). Layer 0 = roots, layer N = nodes whose longest path from a root is N edges.
    pub async fn nodes_layered(&self, pipeline_id: &str) -> Result<Vec<Vec<Node>>, anyhow::Error> {
        let nodes = self.nodes(pipeline_id).await?;
        let edges = self.edges(pipeline_id).await?;

        use std::collections::{HashMap, HashSet};

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut successors: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &nodes {
            in_degree.entry(node.node_id.as_str()).or_insert(0);
            successors
                .entry(node.node_id.as_str())
                .or_insert_with(Vec::new);
        }

        for edge in &edges {
            *in_degree.entry(edge.to_node_id.as_str()).or_insert(0) += 1;
            successors
                .entry(edge.from_node_id.as_str())
                .or_insert_with(Vec::new)
                .push(edge.to_node_id.as_str());
        }

        let mut layers: Vec<Vec<Node>> = vec![];
        let mut current_layer: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let node_map: HashMap<&str, &Node> =
            nodes.iter().map(|n| (n.node_id.as_str(), n)).collect();
        let mut placed: HashSet<&str> = HashSet::new();

        while !current_layer.is_empty() {
            layers.push(
                current_layer
                    .iter()
                    .map(|id| node_map[id].clone())
                    .collect(),
            );

            let mut next_layer: Vec<&str> = vec![];
            for &id in &current_layer {
                placed.insert(id);
                for &succ in successors.get(id).unwrap_or(&vec![]) {
                    let deg = in_degree.get_mut(succ).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_layer.push(succ);
                    }
                }
            }

            current_layer = next_layer;
        }

        Ok(layers)
    }

    /// List all edges in a pipeline.
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

    /// List in-progress pipelines (active or unwinding) for a project, ordered by creation time.
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
                    and state in ('active', 'unwinding')
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

    /// Return the most recently finished pipeline for a project (by revision sequence).
    pub async fn latest_finished_pipeline(
        &self,
        project_id: &str,
    ) -> Result<Option<Pipeline>, anyhow::Error> {
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

    /// Return the event log for all nodes in a pipeline, ordered by sequence.
    pub async fn node_events(&self, pipeline_id: &str) -> Result<Vec<NodeEvent>, anyhow::Error> {
        let con = self.db.get().await?;

        let rows = con
            .query(
                "
                select
                    e.node_id,
                    e.to_state,
                    e.reason,
                    e.created_at
                from pipeline_engine_node_event e
                  join pipeline_engine_node n on e.node_id = n.node_id
                where n.pipeline_id = $1
                order by e.event_seq
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        Ok(rows
            .iter()
            .map(|r| NodeEvent {
                node_id: r.get("node_id"),
                to_state: r.get("to_state"),
                reason: r.get("reason"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    /// Return the latest job error for each node in a pipeline (only failed jobs).
    pub async fn node_errors(
        &self,
        pipeline_id: &str,
    ) -> Result<std::collections::HashMap<String, String>, anyhow::Error> {
        let con = self.db.get().await?;

        let rows = con
            .query(
                "
                select distinct on (j.node_id)
                    j.node_id,
                    j.error
                from pipeline_engine_job j
                  join pipeline_engine_node n on j.node_id = n.node_id
                where n.pipeline_id = $1
                  and j.error is not null
                order by j.node_id, j.created_at desc
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {pipeline_id}", function_name!()))?;

        Ok(rows
            .iter()
            .map(|r| (r.get::<_, String>("node_id"), r.get::<_, String>("error")))
            .collect())
    }

    /// Return the most recent job for a node, or None if never dispatched.
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

    /// Re-attempt a node that reached a terminal failure state.
    ///
    /// - `failed` (forward dispatch failed): resets to `executable` so the next tick
    ///   re-dispatches it for forward work.
    /// - `unwind_failed` (unwind dispatch failed): resets to `finished` so the unwind
    ///   query picks it up again as an unwind-eligible node.
    ///
    /// Returns an error if the node is in any other state.
    pub async fn retry(&self, node_id: &str) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let current_state: Option<String> = tx
            .query_opt(
                "select state from pipeline_engine_node where node_id = $1",
                &[&node_id],
            )
            .await
            .with_context(|| format!("{}: lookup {node_id}", function_name!()))?
            .map(|r| r.get("state"));

        let target_state = match current_state.as_deref() {
            Some("failed") => "executable",
            Some("unwind_failed") => "finished",
            Some(other) => anyhow::bail!("node {node_id} is in state '{other}', cannot retry"),
            None => anyhow::bail!("node {node_id} not found"),
        };

        tx.execute(
            &format!(
                "
                update pipeline_engine_node
                set
                    state = '{target_state}',
                    started_at = null,
                    finished_at = null,
                    updated_at = now()
                where
                    node_id = $1
                "
            ),
            &[&node_id],
        )
        .await
        .with_context(|| format!("{}: {node_id}", function_name!()))?;

        log_node_transition(&tx, node_id, target_state, "retry").await?;

        tx.commit().await?;

        self.assert_invariants().await?;
        Ok(())
    }

    async fn build_unwind_dispatches(
        &self,
        nodes: Vec<Node>,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        if nodes.is_empty() {
            return Ok(vec![]);
        }

        let con = self.db.get().await?;
        let mut dispatches = Vec::with_capacity(nodes.len());

        for node in nodes {
            let restore_revision_id = match &node.resource_key {
                Some(resource_key) => {
                    let row = con
                        .query_opt(
                            "
                            select p.revision_id
                            from pipeline_engine_node n
                              join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                              join revision r on p.revision_id = r.revision_id
                            where n.resource_key = $1
                              and n.state = 'finished'
                              and r.revision_seq < (
                                select r2.revision_seq
                                from pipeline_engine_node n2
                                  join pipeline_engine_pipeline p2 on n2.pipeline_id = p2.pipeline_id
                                  join revision r2 on p2.revision_id = r2.revision_id
                                where n2.node_id = $2
                              )
                            order by r.revision_seq desc
                            limit 1
                            ",
                            &[resource_key, &node.node_id],
                        )
                        .await
                        .with_context(|| {
                            format!("build_unwind_dispatches: resource={resource_key} node={}", node.node_id)
                        })?;
                    row.map(|r| r.get::<_, String>("revision_id"))
                }
                None => None,
            };

            dispatches.push(Dispatch {
                direction: DispatchDirection::Unwind {
                    restore_revision_id,
                },
                revision_id: node.revision_id.clone(),
                node,
            });
        }

        Ok(dispatches)
    }

    #[tracing::instrument(skip_all)]
    async fn complete_unwinding_pipelines(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<(), anyhow::Error> {
        // An unwinding pipeline is complete when:
        // - No nodes are in_progress (all dispatched work has finished)
        // - No unwind-eligible nodes remain (nothing left to dispatch)
        let eligible = self.query_unwind_eligible(tx).await?;
        if eligible.is_empty() {
            let completed: Vec<String> = tx
                .query(
                    "
                    update pipeline_engine_pipeline
                    set state = 'unwound', finished_at = now(), updated_at = now()
                    where state = 'unwinding'
                      and not exists (
                        select 1 from pipeline_engine_node n
                        where n.pipeline_id = pipeline_engine_pipeline.pipeline_id
                          and n.state = 'in_progress'
                      )
                    returning pipeline_id
                    ",
                    &[],
                )
                .await
                .with_context(|| format!("{}", function_name!()))?
                .iter()
                .map(|r| r.get("pipeline_id"))
                .collect();

            for pipeline_id in &completed {
                log_pipeline_transition(tx, pipeline_id, "unwound", "unwind_complete").await?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn detect_and_trigger_unwind(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<(), anyhow::Error> {
        let triggered: Vec<String> = tx
            .query(
                "
                update pipeline_engine_pipeline
                set state = 'unwinding', updated_at = now()
                where state = 'active'
                  and exists (
                    select 1 from pipeline_engine_node n
                    where n.pipeline_id = pipeline_engine_pipeline.pipeline_id
                      and n.state = 'failed'
                      and n.unwind_on_failure = true
                  )
                returning pipeline_id
                ",
                &[],
            )
            .await
            .with_context(|| format!("{}", function_name!()))?
            .iter()
            .map(|r| r.get("pipeline_id"))
            .collect();

        for pipeline_id in &triggered {
            log_pipeline_transition(&tx, pipeline_id, "unwinding", "auto_unwind").await?;
            info!(pipeline_id = %pipeline_id, "auto-unwind triggered");

            // Cancel pending/executable nodes in the full unwind scope.
            // Scope = backward from failed node + forward cascade from all nodes in scope.
            let cancelled: Vec<String> = tx
                .query(
                    "
                    with recursive
                    backward_scope as (
                        select n.node_id
                        from pipeline_engine_node n
                        where n.pipeline_id = $1
                          and n.state not in ('pending', 'executable', 'cancelled')
                          and not exists (
                            select 1 from pipeline_engine_edge e
                              join pipeline_engine_node succ on e.to_node_id = succ.node_id
                            where e.from_node_id = n.node_id
                              and succ.pipeline_id = n.pipeline_id
                              and succ.state not in ('pending', 'executable', 'cancelled')
                          )

                        union

                        select pred.node_id
                        from pipeline_engine_edge e
                          join pipeline_engine_node pred on e.from_node_id = pred.node_id
                          join backward_scope bs on e.to_node_id = bs.node_id
                        where pred.is_unwind_boundary = false
                          and pred.pipeline_id = $1
                    ),
                    forward_cascade as (
                        select node_id from backward_scope

                        union

                        select succ.node_id
                        from pipeline_engine_edge e
                          join pipeline_engine_node succ on e.to_node_id = succ.node_id
                          join forward_cascade fc on e.from_node_id = fc.node_id
                        where succ.pipeline_id = $1
                    )
                    update pipeline_engine_node
                    set state = 'cancelled', updated_at = now()
                    where pipeline_id = $1
                      and node_id in (select node_id from forward_cascade)
                      and state in ('pending', 'executable')
                    returning node_id
                    ",
                    &[pipeline_id],
                )
                .await
                .with_context(|| format!("{}: cancel scope nodes {pipeline_id}", function_name!()))?
                .iter()
                .map(|r| r.get("node_id"))
                .collect();

            let cancelled_refs: Vec<&str> = cancelled.iter().map(|s| s.as_str()).collect();
            log_node_transitions_bulk(&tx, &cancelled_refs, "cancelled", "unwind_scope_cancel")
                .await?;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn query_unwind_eligible(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<Node>, anyhow::Error> {
        let rows = tx
            .query(
                "
                with recursive unwind_scope as (
                    select n.node_id, n.pipeline_id
                    from pipeline_engine_node n
                      join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                    where p.state = 'unwinding'
                      and n.state not in ('pending', 'executable', 'cancelled')
                      and not exists (
                        select 1 from pipeline_engine_edge e
                          join pipeline_engine_node succ on e.to_node_id = succ.node_id
                        where e.from_node_id = n.node_id
                          and succ.pipeline_id = n.pipeline_id
                          and succ.state not in ('pending', 'executable', 'cancelled')
                      )

                    union

                    select pred.node_id, pred.pipeline_id
                    from pipeline_engine_edge e
                      join pipeline_engine_node pred on e.from_node_id = pred.node_id
                      join unwind_scope us on e.to_node_id = us.node_id
                      join pipeline_engine_pipeline pred_p on pred.pipeline_id = pred_p.pipeline_id
                      join revision pred_r on pred_p.revision_id = pred_r.revision_id
                    where pred.is_unwind_boundary = false
                      and pred.pipeline_id = us.pipeline_id
                      and not exists (
                        select 1
                        from pipeline_engine_node newer
                          join pipeline_engine_pipeline newer_p on newer.pipeline_id = newer_p.pipeline_id
                          join revision newer_r on newer_p.revision_id = newer_r.revision_id
                        where newer.resource_key = pred.resource_key
                          and newer.resource_key is not null
                          and newer.state = 'finished'
                          and newer_r.revision_seq > pred_r.revision_seq
                      )
                )
                select
                    n.node_id,
                    p.revision_id,
                    n.event,
                    n.state,
                    n.resource_key
                from pipeline_engine_node n
                  join pipeline_engine_pipeline p on n.pipeline_id = p.pipeline_id
                  join unwind_scope us on n.node_id = us.node_id
                where n.state in ('finished', 'failed')
                  and not exists (
                    select 1
                    from pipeline_engine_edge e
                      join pipeline_engine_node succ on e.to_node_id = succ.node_id
                      join unwind_scope us2 on succ.node_id = us2.node_id
                    where e.from_node_id = n.node_id
                      and succ.state not in ('unwound', 'cancelled')
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
                revision_id: row.get("revision_id"),
                event: row.get("event"),
                state: row.get("state"),
                resource_key: row.get("resource_key"),
            })
            .collect();

        Ok(nodes)
    }

    #[tracing::instrument(skip_all)]
    async fn claim_unwind_nodes(
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
                    finished_at = null,
                    updated_at = now()
                where
                    node_id = any($1)
                    and state in ('finished', 'failed')
                ",
                &[&node_ids],
            )
            .await
            .with_context(|| format!("{}", function_name!()))?;

        assert_all_nodes_claimed(node_ids.len(), claimed);

        log_node_transitions_bulk(tx, node_ids, "in_progress", "claim_unwind").await?;

        Ok(())
    }

    /// Permanently abandon a pipeline. All non-finished nodes become `cancelled`,
    /// pipeline state becomes `cancelled`. Irreversible — there is no un-cancel.
    /// Cancelled nodes are terminal for FIFO, so newer revisions are unblocked.
    pub async fn cancel(&self, pipeline_id: &str) -> Result<(), anyhow::Error> {
        self.assert_invariants().await?;

        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let cancelled: Vec<String> = tx
            .query(
                "
                update pipeline_engine_node
                set
                    state = 'cancelled',
                    updated_at = now()
                where
                    pipeline_id = $1
                    and state != 'finished'
                returning node_id
                ",
                &[&pipeline_id],
            )
            .await
            .with_context(|| format!("{}: cancel nodes {pipeline_id}", function_name!()))?
            .iter()
            .map(|r| r.get("node_id"))
            .collect();

        let cancelled_refs: Vec<&str> = cancelled.iter().map(|s| s.as_str()).collect();
        log_node_transitions_bulk(&tx, &cancelled_refs, "cancelled", "cancel").await?;

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

        log_pipeline_transition(&tx, pipeline_id, "cancelled", "cancel").await?;

        tx.commit().await?;

        self.assert_invariants().await?;
        Ok(())
    }
}
