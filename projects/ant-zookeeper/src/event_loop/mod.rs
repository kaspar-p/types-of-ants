use std::collections::{HashMap, HashSet, VecDeque};

use ant_zookeeper_db::{AntZooStorageClient, Revision};
use tracing::info;

use crate::event_loop::transition::{
    after, is_deployment_complete, is_doable, transition, DeploymentEvent, DeploymentTarget,
    EventName, PipelineError,
};

mod deploy;
pub mod perform;
mod replicate;
pub mod transition;

pub async fn drive_revisions(
    db: &AntZooStorageClient,
    _iterations: i32,
) -> Result<(), PipelineError> {
    let revisions = {
        let mut grouped_revisions: HashMap<String, Vec<Revision>> = HashMap::new();
        for revision in db.list_revisions().await? {
            let pipeline = db
                .get_deployment_pipeline_by_project(&revision.project_id)
                .await?
                .unwrap();

            match grouped_revisions.get_mut(&pipeline) {
                None => {
                    grouped_revisions.insert(pipeline, vec![revision]);
                }
                Some(revs) => {
                    revs.push(revision);
                }
            }
        }
        grouped_revisions
    };

    for (deployment_pipeline_id, pipeline_revisions) in revisions {
        info!(
            "[p={deployment_pipeline_id}] Driving pipeline {} revisions.",
            pipeline_revisions.len()
        );
        drive_pipeline(db, &deployment_pipeline_id, pipeline_revisions).await?;
    }

    Ok(())
}

/// The set of actions currently "ready to go" for the given revision in the pipeline.
///
/// Formally, they are the steps `a` such that `next(s) = a`, where `s` is a completed event for that revision.
async fn revision_frontier(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    revision: &str,
) -> Result<Vec<DeploymentEvent>, PipelineError> {
    let event = DeploymentEvent(
        revision.to_string(),
        DeploymentTarget::Pipeline(deployment_pipeline_id.to_string()),
        EventName::PipelineStarted,
    );

    let mut frontier: Vec<DeploymentEvent> = Vec::new();

    let mut seen: HashSet<DeploymentEvent> = HashSet::new();
    let mut queue: VecDeque<DeploymentEvent> = VecDeque::new();
    seen.insert(event.clone());
    queue.push_back(event.clone());

    loop {
        if let Some(event) = queue.pop_front() {
            for next_event in transition(&db, deployment_pipeline_id, &event).await?.next {
                if is_deployment_complete(db, &next_event).await? {
                    if !seen.contains(&next_event) {
                        queue.push_back(next_event.clone());
                        seen.insert(next_event.clone());
                    }
                } else {
                    frontier.push(next_event);
                }
            }
        } else {
            break;
        }
    }

    return Ok(frontier);
}

/// The set of events that are "ready to go" for the pipeline, for all revisions.
///
/// Formally, the events `a` such that `next(s) = a` where `s` is a completed event.
///
/// Returns a set like [(r1, "pipe-1", "pipeline-finished"), (r2, "pipe-1", "pipeline-started")]
/// meaning that there's an older revision (r1) that's nearly done, only the "pipeline-finished" event
/// is remaining for it. There's also a newer revision (r2) that just started, and gets its first event.
///
/// There may be multiple frontier events for a single revision, since single events can "fan out" to others.
/// For example, a single "host-group-started" fans out to many different "host-started" events, one for each
/// host in the host group.
///
/// * revisions: The revisions in this pipeline, finished and in progress.
///              Index 0 is the NEWEST revision, so earlier in vec == more modern
async fn frontier(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    pipeline_revisions: &Vec<Revision>,
) -> Result<Vec<DeploymentEvent>, PipelineError> {
    let frontiers = {
        let mut frontiers: Vec<(&Revision, Vec<DeploymentEvent>)> = vec![]; // (revision_id, events)
        for revision in pipeline_revisions {
            frontiers.push((
                &revision,
                revision_frontier(db, deployment_pipeline_id, &revision.id).await?,
            ));
        }
        frontiers
    };

    info!(
        "Frontiers Pre-Filtering ({}) {:?}",
        frontiers.len(),
        frontiers
    );

    let mut filtered_frontiers: Vec<DeploymentEvent> = vec![];

    // If any events from AFTER(a) were completed by newer revisions, then a should be removed from the
    // frontier, it's been surpassed.

    for (rev_a, frontier_a) in frontiers.iter().rev() {
        let mut new_frontier_a = vec![];

        for event_a in frontier_a {
            let mut on_frontier = true;

            let after_events = after(db, deployment_pipeline_id, &event_a).await?;
            for after_event_a in after_events {
                for rev_b in pipeline_revisions {
                    if rev_a.seq >= rev_b.seq {
                        continue;
                    }

                    let after_event_b = after_event_a.for_other_revision(rev_b.id.to_string());
                    if is_deployment_complete(db, &after_event_b).await? {
                        // Some event that comes after this "frontier" event for revision A has already been
                        // completed, so this "frontier" event has been surpassed.
                        // For example, rev A, event_a could be "host-artifact-replicated" and might have failed
                        // but then rev B fixes the bug and gets to "pipeline-finished" all the way.
                        on_frontier = false;
                        break;
                    }
                }

                if !on_frontier {
                    break;
                }
            }

            if on_frontier {
                new_frontier_a.push(event_a.clone());
            }
        }

        filtered_frontiers.append(&mut new_frontier_a);
    }

    info!(
        "Frontiers Post-Filtering {} {:?}",
        filtered_frontiers.len(),
        filtered_frontiers
    );

    Ok(filtered_frontiers)
}

async fn drive_pipeline(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    revisions: Vec<Revision>,
) -> Result<(), PipelineError> {
    let mut frontier = frontier(&db, &deployment_pipeline_id, &revisions).await?;

    loop {
        let mut new_frontier = Vec::new();

        for event in &frontier {
            info!(
                "[p={deployment_pipeline_id} v={}] Driving frontier event: {event:?}",
                event.0
            );

            // The deployment may already have been completed by another thread, we want to handle that
            // gracefully since the frontier we have is just in-memory.
            if is_deployment_complete(db, event).await? {
                let mut trans = transition(db, deployment_pipeline_id, &event).await?;
                info!("[p={deployment_pipeline_id} v={}] Event {event:?} is already finished, progressing pipeline with {} new events...", event.0, trans.next.len());
                new_frontier.append(&mut trans.next);
            }

            drive_iteration(db, deployment_pipeline_id, frontier.iter(), &event).await?;
        }

        if new_frontier.len() == 0 {
            break;
        } else {
            frontier = new_frontier;
        }
    }

    Ok(())
}

/// Given the current `event`, this function either does nothing, or schedules a task to be performed
/// if the current event is DOABLE.
async fn drive_iteration<'a, T: Iterator<Item = &'a DeploymentEvent>>(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    frontier: T,
    event: &DeploymentEvent,
) -> Result<(), PipelineError> {
    if !is_doable(&db, deployment_pipeline_id, frontier, &event).await? {
        info!(
            "[p={deployment_pipeline_id} v={}] Event {event:?} is not doable.",
            event.0
        );
        return Ok(());
    }

    let project = db
        .get_project_from_deployment_pipeline(deployment_pipeline_id)
        .await?;

    // Index 0 is newest attempt at the job
    let previous_jobs = db
        .list_deployment_jobs(
            &event.0,
            &project,
            &deployment_pipeline_id,
            &event.1.as_target_type(),
            &event.1.as_target_id(),
            &event.2.to_string(),
        )
        .await?;

    let successful = previous_jobs.iter().find(|(_, _, is_success)| *is_success);
    if let Some(successful) = successful {
        panic!(
            "[p={deployment_pipeline_id} v={}] Previous job {} succeeded, this shouldn't {}",
            "happen!", event.0, successful.0
        );
    }

    let retryable = previous_jobs
        .iter()
        .find(|(_, is_retryable, is_success)| !*is_success && *is_retryable);

    if previous_jobs.len() > 0 && retryable.is_none() {
        info!(
            "[p={deployment_pipeline_id} v={}] All {} prev. jobs [{}] failed + aren't {}",
            "retryable...",
            event.0,
            previous_jobs.len(),
            previous_jobs
                .iter()
                .map(|j| j.0.as_ref())
                .collect::<Vec<&str>>()
                .join(", ")
        );
        return Ok(());
    }

    if let Some(prev) = retryable {
        info!(
            "[p={deployment_pipeline_id} v={}] Prev. job {} failed but is retryable, {}",
            "creating retry and marking previous non-retryable.", event.0, prev.0
        );

        // Set the previous job to non-retryable since we're about to kickoff a retry!
        db.set_deployment_job_retryable(&prev.0, false).await?;
    }

    // Schedule the deployment job to actually happen. A different process/thread/api is always looking for jobs to perform!
    let job_id = db
        .create_deployment_job(
            &event.0,
            &project,
            &deployment_pipeline_id,
            &event.1.as_target_type(),
            &event.1.as_target_id(),
            &event.2.to_string(),
        )
        .await?;

    info!(
        "[p={deployment_pipeline_id} v={}] Scheduled job {job_id} for event {event:?}",
        event.0
    );

    return Ok(());
}
