use tracing::info;

use crate::{
    event_loop::{
        deploy::deploy_artifact,
        replicate::replicate_artifact_step,
        transition::{DeploymentEvent, Event},
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
    event: &DeploymentEvent,
) -> Result<JobCompletion<()>, anyhow::Error> {
    type E = Event;

    match event {
        DeploymentEvent(
            revision,
            E::HostArtifactReplicated {
                host_group_id,
                host,
            },
        ) => {
            info!("Beginning replication of version {revision} to host {host}...");

            let host_group = state
                .db
                .get_host_group_by_id(&host_group_id)
                .await?
                .unwrap();

            replicate_artifact_step(state, &revision, &host_group, &host).await?;

            Ok(JobCompletion::Finished(()))
        }

        DeploymentEvent(
            revision,
            E::HostArtifactDeployed {
                host_group_id,
                host: host_id,
            },
        ) => {
            let host_group = state
                .db
                .get_host_group_by_id(&host_group_id)
                .await?
                .unwrap();

            let version = state.db.get_revision(&revision).await?.version;

            deploy_artifact(state, &host_group.project, &version, &host_id).await?;

            Ok(JobCompletion::Finished(()))
        }

        DeploymentEvent(revision, E::ArtifactRegistered { stage_id, arch }) => {
            let stage = state
                .db
                .get_deployment_pipeline_stage(stage_id)
                .await?
                .unwrap();

            // The project that this build stage (we assume this event was emitted by a build stage) is responsible for building.
            let building_project_id = stage.3.unwrap();

            let missing = state
                .db
                .missing_artifacts_for_revision_id(&building_project_id, &revision)
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
