use anyhow::Context;
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use futures::{future::join_all, TryFutureExt};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    err::AntZookeeperError,
    event_loop::{
        drive_revisions,
        perform::{perform, JobCompletion},
        transition::{DeploymentEvent, PipelineError},
    },
    state::AntZookeeperState,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IteratePipelineResponse {}

async fn iterate_pipeline(
    State(state): State<AntZookeeperState>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    // Schedule any deployment jobs available
    drive_revisions(&state.db, 1)
        .map_err(|e| match e {
            PipelineError::UnknownStep(p) => AntZookeeperError::InternalServerError(Some(
                anyhow::Error::msg(format!("Unknown deployment event: {p}")),
            )),
            PipelineError::DatabaseError(e) => AntZookeeperError::InternalServerError(Some(
                e.context("Pipeline orchestration failure"),
            )),
        })
        .await?;

    let unfinished_jobs = state.db.list_unfinished_deployment_jobs().await?;

    if unfinished_jobs.len() > 0 {
        info!("Processing {} unfinished jobs...", unfinished_jobs.len());
    }

    let handles = unfinished_jobs.into_iter().map(|job| {
        let state = state.clone();
        async move {
            let event = DeploymentEvent(job.revision.clone(), job.event_document.clone().into());

            // Do the work.
            info!("Performing: {event:?}");

            state.db.start_deployment_job(&job.job_id).await?;

            let work: Result<Result<JobCompletion<()>, anyhow::Error>, tokio::task::JoinError> =
                tokio::spawn({
                    let job_id = job.job_id.clone();
                    let state = state.clone();
                    async move {
                        perform(&state, &event).await.with_context(|| {
                            format!(
                                "Failed to perform scheduled deployment job [{}] for event [{}]",
                                job_id, event
                            )
                        })
                    }
                })
                .await;

            let is_success: Option<bool> = match &work {
                Ok(Ok(JobCompletion::Pending)) => None,
                Ok(Ok(JobCompletion::Finished(_))) => Some(true),
                Ok(Err(e)) => {
                    error!("Handler Error: {:?}", e);
                    Some(false)
                }
                Err(e) => {
                    error!("Orchestration Error: {:?}", e);
                    Some(false)
                }
            };

            // If the job finished, set its status and finished_at timestamp.
            if let Some(is_success) = is_success {
                info!(
                    "Job {} complete, success={}",
                    job.job_id.clone(),
                    is_success
                );
                state
                    .db
                    .complete_deployment_job(
                        &job.job_id,
                        &job.revision,
                        &job.event_document,
                        is_success,
                    )
                    .await?;
            } else {
                // Clear the started_at field to set the job back to pending.
                state.db.unstart_deployment_job(&job.job_id).await?;
            }

            return Ok(());
        }
    });

    join_all(handles)
        .await
        .into_iter()
        .collect::<Result<(), anyhow::Error>>()?;

    Ok((StatusCode::OK, Json(IteratePipelineResponse {})))
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new().route_with_tsr("/iteration", post(iterate_pipeline))
}
