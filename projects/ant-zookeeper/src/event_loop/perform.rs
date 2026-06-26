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

pub(crate) async fn host_artifact_replicated(
    state: AntZookeeperState,
    event: DeploymentEvent,
) -> Result<JobCompletion<()>, anyhow::Error> {
    match event {
        DeploymentEvent(
            revision,
            Event::HostArtifactReplicated {
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

            replicate_artifact_step(
                &state,
                &revision,
                &host_group.project,
                &host_group.environment,
                &host,
            )
            .await?;

            Ok(JobCompletion::Finished(()))
        }

        e => panic!("wrong event: {e}"),
    }
}

pub(crate) async fn host_artifact_deployed(
    state: AntZookeeperState,
    event: DeploymentEvent,
) -> Result<JobCompletion<()>, anyhow::Error> {
    match event {
        DeploymentEvent(
            revision,
            Event::HostArtifactDeployed {
                host_group_id,
                host: host_id,
            },
        ) => {
            let host_group = state
                .db
                .get_host_group_by_id(&host_group_id)
                .await?
                .unwrap();

            let host = state.db.get_host(&host_id).await?.unwrap();

            let (_, version, _) = state
                .db
                .get_artifact_by_revision(&revision, &host_group.project, Some(&host.1))
                .await?
                .unwrap();

            deploy_artifact(&state, &host_group.project, &version, &host_id).await?;

            Ok(JobCompletion::Finished(()))
        }
        e => panic!("wrong event: {e}"),
    }
}

pub(crate) async fn artifact_registered(
    state: AntZookeeperState,
    event: DeploymentEvent,
) -> Result<JobCompletion<()>, anyhow::Error> {
    match event {
        DeploymentEvent(revision, Event::ArtifactRegistered { arch, .. }) => {
            let missing = state
                .db
                .missing_architectures_for_revision(&revision)
                .await?;

            if missing.contains(&arch) {
                info!("Still missing {arch:?} on {revision}, stay pending.");
                return Ok(JobCompletion::Pending);
            } else {
                info!("Architecture {arch:?} has been registered on {revision}.");
                return Ok(JobCompletion::Finished(()));
            }
        }
        e => panic!("wrong event: {e}"),
    }
}

pub(crate) async fn stage_finished(
    state: AntZookeeperState,
    event: DeploymentEvent,
) -> Result<JobCompletion<()>, anyhow::Error> {
    match event {
        DeploymentEvent(revision, Event::StageFinished { stage_id }) => {
            let stage = state
                .db
                .get_deployment_pipeline_stage(&stage_id)
                .await?
                .unwrap();

            match stage.2.as_str() {
                // Build stages aren't done even if their underlying artifact-registrations are done
                // because there may be version mismatches. So we are Pending until not only we have
                // all the host architectures we need (x86, ...), but also that each of those are the same version.
                //
                // All of that criteria is the "activated" state of the revision, so we just check that.
                "build" => {
                    let revision = state.db.get_revision(&revision).await?.unwrap();
                    if revision.activated_at.is_some() {
                        return Ok(JobCompletion::Finished(()));
                    } else {
                        return Ok(JobCompletion::Pending);
                    }
                }

                // Deployment stages are done immediately, nothing to do.
                _ => return Ok(JobCompletion::Finished(())),
            }
        }
        e => panic!("wrong event: {e}"),
    }
}

// / Runs inside the worker thread, actually performs the work of a task. This could be:
// /   1. Running the actual work, e.g. taking an artifact and repackaging it for a host, replicating it to a
// /         host, etc.
// /   2. Completing instantly. Most jobs like "stage-finished" have no real work attached for now.
// /         But in the future, attaching "time blockers" where stages only start from 9am - 5pm would be
// /         very easy, so they are good hooks.
// /   3. Checking some external database or state and returning its status. Since the pipeline often cannot
// /         MAKE work happen, the scheduled job can often just be relied upon to periodically check a status.
// /         The build steps do this for registering architectures, they just wait until all known architectures
// /         are registered.
// pub async fn perform(
//     state: &AntZookeeperState,
//     event: &DeploymentEvent,
// ) -> Result<JobCompletion<()>, anyhow::Error> {
//     type E = Event;

//     match event {
//         // If we didn't understand the event, then there was likely nothing to do for it.
//         e => {
//             info!("Perform default job handling, complete immediately: {e:?}");
//             Ok(JobCompletion::Finished(()))
//         }
//     }
// }
