use tracing::info;

use crate::{
    event_loop::{
        deploy::deploy_artifact,
        replicate::replicate_artifact_step,
        transition::{DeploymentEvent, DeploymentTarget, EventName},
    },
    state::AntZookeeperState,
};

pub enum JobCompletion<T> {
    /// If a job completes in Pending, it can be rescheduled in the future and is effectively "waiting" on some
    /// external condition. The completion of the job is then checked periodically via the scheduled job, until
    /// Finished(T) is returned.
    ///
    /// For example, the build stage would return pending if not all architectures are registered yet,
    /// or any deployment might return pending if the deployment occurs outside of allowed time windows,
    /// if that feature is implemented.
    Pending,

    /// The job was actually completed, and the pipeline should move on.
    Finished(T),
}

/// Runs inside the worker thread, actually performs the work of a task. This could be:
///   1. Running the actual work, e.g. taking an artifact and repackaging it for a host, replicating it to a
///         host, etc.
///   2. Completing instantly. Most jobs like "stage-finished" have no real work attached for now.
///         But in the future, attaching "time blockers" where stages only start from 9am - 5pm would be
///         very easy, so they are good hooks.
///   3. Checking some external database or state and returning its status. Since the pipeline often cannot
///         MAKE work happen, the scheduled job can often just be relied upon to periodically check a status.
///         The build steps do this for registering architectures, they just wait until all known architectures
///         are registered.
pub async fn perform(
    state: &AntZookeeperState,
    deployment_pipeline_id: &str,
    event: &DeploymentEvent,
) -> Result<JobCompletion<()>, anyhow::Error> {
    type T = DeploymentTarget;
    type E = EventName;

    match event {
        DeploymentEvent(revision, T::Host(host), E::HostArtifactReplicated) => {
            info!("Beginning replication of version {revision} to host {host}...");

            let host_group_id = state
                .db
                .get_host_group_by_host(deployment_pipeline_id, &host)
                .await?
                .unwrap();

            let host_group = state
                .db
                .get_host_group_by_id(&host_group_id)
                .await?
                .unwrap();

            let project = state
                .db
                .get_project_from_deployment_pipeline(deployment_pipeline_id)
                .await?;

            replicate_artifact_step(state, &project, &revision, &host_group, &host).await?;

            Ok(JobCompletion::Finished(()))
        }

        DeploymentEvent(revision, T::Host(host), E::HostArtifactDeployed) => {
            let project = state
                .db
                .get_project_from_deployment_pipeline(deployment_pipeline_id)
                .await?;

            let version = state.db.get_revision(&revision).await?.version;

            deploy_artifact(state, &project, &version, &host).await?;

            Ok(JobCompletion::Finished(()))
        }

        DeploymentEvent(revision, T::Stage(_), E::ArtifactArchitectureRegistered(arch)) => {
            let missing = state
                .db
                .missing_artifacts_for_revision_id(&revision)
                .await?;

            if missing.contains(&arch) {
                info!("Still missing {arch:?} on {revision}, stay pending.");
                return Ok(JobCompletion::Pending);
            } else {
                info!("Architecture {arch:?} has been registered on {revision}.");
                return Ok(JobCompletion::Finished(()));
            }
        }

        // If we didn't understand the event, then there was likely nothing to do for it.
        e => {
            info!("Perform default job handling, complete immediately: {e:?}");
            Ok(JobCompletion::Finished(()))
        }
    }
}
