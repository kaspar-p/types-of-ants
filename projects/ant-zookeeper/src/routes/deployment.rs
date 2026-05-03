use anyhow::Context;
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info, info_span, Instrument};

use crate::{
    err::AntZookeeperError,
    event_loop::{
        drive_revisions,
        perform::JobCompletion,
        transition::{self, DeploymentEvent},
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
        .await
        .with_context(|| "Pipeline orchestration failure")?;

    let unfinished_jobs = state.db.list_unfinished_deployment_jobs().await?;

    if unfinished_jobs.len() > 0 {
        info!("Processing {} unfinished jobs...", unfinished_jobs.len());
    }

    let handles = unfinished_jobs.into_iter().map(|job| {
        let state = state.clone();
        async move {
            let event = DeploymentEvent(job.revision.clone(), job.event_document.clone().into());

            let perform_fn = transition::transition(&state.db, &event).await?.perform;

            let is_success = match perform_fn {
                None => {
                    error!(
                        "Deployment job rev={} event={} has no perform, but ignoring...",
                        event.0,
                        event.1.to_string()
                    );

                    // Just marking it complete, shouldn't have happened though
                    Some(true)
                }

                Some(perform_fn) => {
                    // Do the work.

                    state.db.start_deployment_job(&job.job_id).await?;

                    let work: Result<
                        Result<JobCompletion<()>, anyhow::Error>,
                        tokio::task::JoinError,
                    > = tokio::spawn({
                        let job_id = job.job_id.clone();
                        let state = state.clone();
                        let event2 = event.clone();
                        let event3 = event.clone();
                        let span = info_span!("job", job_id = job_id);
                        async move {
                            perform_fn(&state, event2).await.with_context(|| {
                                format!(
                                "Failed to perform scheduled deployment job [{}] for event [{}]",
                                job_id, event3
                            )
                            })
                        }
                        .instrument(span)
                    })
                    .await;

                    let is_success: Option<bool> = match &work {
                        Ok(Ok(JobCompletion::Pending)) => None,
                        Ok(Ok(JobCompletion::Finished(_))) => {
                            info!("Completed [{}] for {event}", job.job_id);
                            Some(true)
                        }
                        Ok(Err(e)) => {
                            error!("Handler Error: {:?}", e);
                            Some(false)
                        }
                        Err(e) => {
                            error!("Orchestration Error: {:?}", e);
                            Some(false)
                        }
                    };

                    is_success
                }
            };

            // If the job finished, set its status and finished_at timestamp.
            if let Some(is_success) = is_success {
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryJobRequest {
    pub job_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryJobResponse {
    pub retried_at: DateTime<Utc>,
}

async fn retry_job(
    State(state): State<AntZookeeperState>,
    Json(req): Json<RetryJobRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    match state.db.get_deployment_job(&req.job_id).await? {
        None => {
            return Err(AntZookeeperError::ResourceNotFound(req.job_id));
        }
        Some((_, is_success, is_retryable, updated_at, _, _)) => {
            if is_success {
                return Err(AntZookeeperError::ValidationError(format!(
                    "Job {} has already been completed and cannot be retried.",
                    req.job_id
                )));
            }

            if is_retryable {
                return Ok((
                    StatusCode::OK,
                    Json(RetryJobResponse {
                        retried_at: updated_at,
                    }),
                ));
            }
        }
    }

    let updated_at = state
        .db
        .set_deployment_job_retryable(&req.job_id, true)
        .await?;

    return Ok((
        StatusCode::OK,
        Json(RetryJobResponse {
            retried_at: updated_at,
        }),
    ));
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new()
        .route_with_tsr("/iteration", post(iterate_pipeline))
        .route_with_tsr("/retry", post(retry_job))
}
