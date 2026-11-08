//! Frees space on storage nodes. Deleting a key only marks its object deletable, so this purges
//! the shards of objects that no key points to, and blobs that no placement points to (left by
//! failed uploads).
//!
//! It's a reconciliation loop: each pass moves the nodes towards what the database says should
//! be on them, and anything it can't do (a node is offline, data is still inside the grace
//! period) is picked up by a later pass.

use std::collections::{HashMap, HashSet};

use axum::{extract::State, routing::post, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tracing::{info, warn};

use crate::{
    auth::BearerClaims,
    placement::{resolve_storage_node_clients, HashRingNode},
    AntArchiveError, AntArchiveState,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct VacuumReport {
    /// Another instance holds the vacuum lease.
    pub already_running: bool,
    /// Nothing changed since the last vacuum, and it left nothing to retry.
    pub skipped: bool,
    pub placements_purged: usize,
    pub orphans_deleted: usize,
    /// Unreferenced placements and blobs still inside the grace period.
    pub deferred: usize,
    /// Deletes that failed. The next vacuum retries them.
    pub failures: Vec<String>,
}

const LIST_PAGE_SIZE: usize = 1000;

const JOB_ID: &str = "vacuum";

/// Long enough for a pass to finish. If an instance dies mid-pass, another takes over after
/// this.
const LEASE_SECONDS: i64 = 60 * 60;

pub async fn vacuum(state: &AntArchiveState) -> Result<VacuumReport, AntArchiveError> {
    let Some(lease) = state
        .db
        .acquire_job_lease(JOB_ID, &state.instance_id, LEASE_SECONDS)
        .await?
    else {
        return Ok(VacuumReport {
            already_running: true,
            ..Default::default()
        });
    };

    // Read before doing anything, so activity during this pass makes the next one run.
    let activity = state.db.last_activity().await?;
    if lease.last_run_clean && lease.last_activity == activity {
        state
            .db
            .release_job_lease(JOB_ID, &state.instance_id, activity, true)
            .await?;
        return Ok(VacuumReport {
            skipped: true,
            ..Default::default()
        });
    }

    let result = run_pass(state).await;

    // A pass that errored part-way isn't clean, so the next one doesn't skip.
    let clean = matches!(&result, Ok(r) if r.deferred == 0 && r.failures.is_empty());
    state
        .db
        .release_job_lease(JOB_ID, &state.instance_id, activity, clean)
        .await?;

    result
}

async fn run_pass(state: &AntArchiveState) -> Result<VacuumReport, AntArchiveError> {
    let mut report = VacuumReport::default();
    let nodes: HashMap<String, HashRingNode> = resolve_storage_node_clients(state)
        .await?
        .into_iter()
        .map(|n| (n.node_id.clone(), n))
        .collect();

    purge_unreferenced_placements(state, &nodes, &mut report).await?;
    for node in nodes.values() {
        delete_orphans(state, node, &mut report).await?;
    }

    info!(
        "Vacuum purged {} placements and {} orphans, deferred {}, {} failures",
        report.placements_purged,
        report.orphans_deleted,
        report.deferred,
        report.failures.len()
    );

    Ok(report)
}

async fn purge_unreferenced_placements(
    state: &AntArchiveState,
    nodes: &HashMap<String, HashRingNode>,
    report: &mut VacuumReport,
) -> Result<(), AntArchiveError> {
    let placements = state
        .db
        .list_unreferenced_placements(state.vacuum_grace.num_seconds())
        .await?;

    for placement in placements {
        if !placement.due {
            report.deferred += 1;
            continue;
        }

        let Some(node) = nodes.get(&placement.storage_node_id) else {
            let failure = format!(
                "ANT-ERR-145: node {} for placement {} is not in service discovery",
                placement.storage_node_id, placement.placement_id
            );
            warn!("{failure}");
            report.failures.push(failure);
            continue;
        };

        // Already gone (Ok(false)) is as good as deleted.
        match node.client.delete(&placement.storage_key).await {
            Ok(_) => {
                state
                    .db
                    .mark_placement_purged(&placement.placement_id)
                    .await?;
                report.placements_purged += 1;
            }
            Err(e) => {
                let failure = format!(
                    "ANT-ERR-146: deleting {} from {}: {e}",
                    placement.storage_key,
                    node.to_string()
                );
                warn!("{failure}");
                report.failures.push(failure);
            }
        }
    }

    Ok(())
}

async fn delete_orphans(
    state: &AntArchiveState,
    node: &HashRingNode,
    report: &mut VacuumReport,
) -> Result<(), AntArchiveError> {
    // Fetched before listing, so a blob placed during the listing is newer than the grace
    // period rather than missing from this set.
    let placed: HashSet<String> = state
        .db
        .list_storage_keys_on_node(&node.node_id)
        .await?
        .iter()
        .map(|key| base16ct::lower::encode_string(&Sha256::digest(key.as_bytes())))
        .collect();
    let cutoff = Utc::now() - state.vacuum_grace;

    let mut after: Option<String> = None;
    loop {
        let page = match node
            .client
            .list_blobs(after.as_deref(), LIST_PAGE_SIZE)
            .await
        {
            Ok(page) => page,
            Err(e) => {
                let failure = format!("ANT-ERR-147: listing {}: {e}", node.to_string());
                warn!("{failure}");
                report.failures.push(failure);
                return Ok(());
            }
        };

        for blob in page.blobs {
            if placed.contains(&blob.hash) {
                continue;
            }
            // An upload writes its blob before recording the placement.
            if blob.modified >= cutoff {
                report.deferred += 1;
                continue;
            }

            match node.client.delete_blob_by_hash(&blob.hash).await {
                Ok(_) => report.orphans_deleted += 1,
                Err(e) => {
                    let failure = format!(
                        "ANT-ERR-148: deleting orphan {} from {}: {e}",
                        blob.hash,
                        node.to_string()
                    );
                    warn!("{failure}");
                    report.failures.push(failure);
                }
            }
        }

        match page.next {
            Some(next) => after = Some(next),
            None => return Ok(()),
        }
    }
}

async fn post_vacuum(
    State(state): State<AntArchiveState>,
    _auth: BearerClaims,
) -> Result<Json<VacuumReport>, AntArchiveError> {
    Ok(Json(vacuum(&state).await?))
}

pub fn make_routes(state: AntArchiveState) -> Router {
    use ant_library::routes::Routes;

    Routes::new()
        .post("/", post(post_vacuum))
        .build()
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                ))),
        )
}
