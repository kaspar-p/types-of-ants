use ant_library::routes::Routes;
use anyhow::Context;
use axum::{extract::State, response::IntoResponse, routing::post, Json};
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
    pipeline::dispatch::dispatch,
    state::AntZookeeperState,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IteratePipelineResponse {}

async fn iterate_pipeline(
    State(state): State<AntZookeeperState>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    // New pipeline engine tick
    let state_for_dispatch = state.clone();
    let tick_handle = state
        .engine
        .tick(move |d| {
            let state = state_for_dispatch.clone();
            async move { dispatch(state, d).await }
        })
        .await
        .with_context(|| "Pipeline engine tick failure")?;
    tick_handle
        .join()
        .await
        .with_context(|| "Pipeline engine join failure")?;

    // Legacy: Schedule any deployment jobs available
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
                                    "Failed to perform scheduled deployment job [{}] for event \
                                     [{}]",
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
pub struct RetryRequest {
    pub job_id: Option<String>,
    pub node_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryResponse {
    pub retried_at: DateTime<Utc>,
}

async fn retry_job(
    State(state): State<AntZookeeperState>,
    Json(req): Json<RetryRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    // New pipeline engine: retry by node_id
    if let Some(node_id) = &req.node_id {
        state
            .engine
            .retry(node_id)
            .await
            .map_err(|e| AntZookeeperError::ValidationError(e.to_string()))?;
        return Ok((
            StatusCode::OK,
            Json(RetryResponse {
                retried_at: Utc::now(),
            }),
        ));
    }

    // Legacy system: retry by job_id
    let job_id = req.job_id.as_deref().ok_or_else(|| {
        AntZookeeperError::ValidationError("either jobId or nodeId is required".to_string())
    })?;

    match state.db.get_deployment_job(job_id).await? {
        None => {
            return Err(AntZookeeperError::ResourceNotFound(job_id.to_string()));
        }
        Some((_, is_success, is_retryable, updated_at, _, _)) => {
            if is_success {
                return Err(AntZookeeperError::ValidationError(format!(
                    "Job {} has already been completed and cannot be retried.",
                    job_id
                )));
            }

            if is_retryable {
                return Ok((
                    StatusCode::OK,
                    Json(RetryResponse {
                        retried_at: updated_at,
                    }),
                ));
            }
        }
    }

    let updated_at = state.db.set_deployment_job_retryable(job_id, true).await?;

    Ok((
        StatusCode::OK,
        Json(RetryResponse {
            retried_at: updated_at,
        }),
    ))
}

pub fn routes() -> Routes<AntZookeeperState> {
    Routes::new()
        .post("/iteration", post(iterate_pipeline))
        .post("/retry", post(retry_job))
}
