use axum::{extract::State, response::IntoResponse, routing::post, Router};
use axum_extra::routing::RouterExt;
use futures::{future::join_all, TryFutureExt};
use http::StatusCode;
use tracing::info;

use crate::{
    err::AntZookeeperError,
    event_loop::{
        drive_revisions,
        transition::{perform, DeploymentEvent, DeploymentTarget, JobCompletion, PipelineError},
    },
    state::AntZookeeperState,
};

async fn iterate_pipeline(
    State(state): State<AntZookeeperState>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    // Schedule any deployment jobs available
    info!("Scheduling new tasks.");
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

    info!("Processing {} unfinished jobs...", unfinished_jobs.len());

    let handles = unfinished_jobs.into_iter().map(|job| {
        let event = DeploymentEvent(
            job.revision.clone(),
            DeploymentTarget::from_strings(&job.target_type, job.target_id.clone()),
            job.event_name.clone().into(),
        );

        let state2 = state.clone();
        tokio::spawn(async move {
            // Do the work.
            info!("Performing: {event:?}");
            let work = perform(&state2, &job.deployment_pipeline_id.clone(), event).await;

            let is_success: Option<bool> = match &work {
                Ok(JobCompletion::Pending) => None,
                Ok(JobCompletion::Finished(_)) => Some(true),
                Err(_) => Some(false),
            };

            // Record it.
            if let Some(is_success) = is_success {
                info!("Marking {} complete is_success={}", &job.job_id, is_success);
                state2
                    .db
                    .complete_deployment_job(
                        &job.job_id,
                        &job.revision,
                        &job.target_id,
                        &job.event_name,
                        is_success,
                    )
                    .await?;
            }

            work?; // Exit if error

            return Ok(());
        })
    });

    join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<Result<(), anyhow::Error>>, tokio::task::JoinError>>()?
        .into_iter()
        .collect::<Result<Vec<()>, anyhow::Error>>()?;

    Ok((StatusCode::OK, "Pipeline iteration complete."))
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new().route_with_tsr("/iteration", post(iterate_pipeline))
}
