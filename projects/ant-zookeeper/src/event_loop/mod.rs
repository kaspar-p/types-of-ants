use ant_zoo_storage::AntZooStorageClient;
use tracing::info;

use crate::event_loop::transition::{
    frontier, is_deployment_complete, is_doable, transition, DeploymentEvent, EventName,
    PipelineError,
};

mod deploy;
mod replicate;
pub mod transition;

pub async fn drive_revisions(
    db: &AntZooStorageClient,
    iterations: i32,
) -> Result<(), PipelineError> {
    for (revision, deployment_pipeline_id) in db
        .list_revisions_missing_event(&EventName::PipelineFinished.to_string())
        .await?
    {
        info!(
            "Driving pipeline {deployment_pipeline_id} version {revision} x{iterations} times..."
        );
        // assume most pipelines end-to-end generate < 10k events
        drive_pipeline(db, &revision, &deployment_pipeline_id).await?;
    }

    Ok(())
}

async fn drive_pipeline(
    db: &AntZooStorageClient,
    revision: &str,
    deployment_pipeline_id: &str,
) -> Result<(), PipelineError> {
    let mut frontier = Vec::from(frontier(&db, &revision, &deployment_pipeline_id).await?);
    info!(
        "[p={deployment_pipeline_id} v={revision}] Frontier calculated, size={}",
        frontier.len()
    );

    loop {
        let mut new_frontier = Vec::new();

        for event in &frontier {
            info!("Driving frontier event: {event:?}");
            let mut requeue_events =
                drive_iteration(db, deployment_pipeline_id, frontier.iter(), &event).await?;

            info!(
                "Event {event:?} fans out to {} new events: {:?}",
                requeue_events.len(),
                requeue_events
            );
            new_frontier.append(&mut requeue_events);
        }

        if new_frontier.len() == 0 {
            break;
        } else {
            frontier = new_frontier;
        }
    }

    Ok(())
}

async fn drive_iteration<'a, T: Iterator<Item = &'a DeploymentEvent>>(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    frontier: T,
    event: &DeploymentEvent,
) -> Result<Vec<DeploymentEvent>, PipelineError> {
    if !is_doable(&db, deployment_pipeline_id, frontier, &event).await? {
        info!("Event {event:?} is not doable.");
        return Ok(vec![]);
    }

    let transition = transition(db, deployment_pipeline_id, &event).await?;

    if is_deployment_complete(db, event).await? {
        info!("Event {event:?} is already finished, progressing pipeline...");
        return Ok(transition.next);
    }

    let project = db
        .get_project_from_deployment_pipeline(deployment_pipeline_id)
        .await?;

    let previous = db
        .get_job_previously_failed(
            &event.0,
            &project,
            &deployment_pipeline_id,
            &event.1.as_target_type(),
            &event.1.as_target_id(),
            &event.2.to_string(),
        )
        .await?;
    if let Some(prev) = previous {
        if !prev.1 {
            info!("Previous job {} failed, and is not retryable...", prev.0);
            return Ok(vec![]);
        } else {
            info!("Previous job {} failed but is retryable...", prev.0);
        }
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

    info!("Scheduled job: {job_id} for event {event:?}");

    Ok(vec![])
}
