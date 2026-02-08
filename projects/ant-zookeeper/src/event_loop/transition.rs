use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
    str::FromStr,
};

use ant_library::host_architecture::HostArchitecture;
use ant_zookeeper_db::AntZooStorageClient;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    event_loop::{deploy::deploy_artifact, replicate::replicate_artifact_step},
    state::AntZookeeperState,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentTarget {
    Pipeline(String),
    Stage(String),
    HostGroup(String),
    Host(String),
}

impl DeploymentTarget {
    pub fn from_strings(target_type: &str, target_id: String) -> DeploymentTarget {
        match target_type {
            "pipeline" => DeploymentTarget::Pipeline(target_id),
            "stage" => DeploymentTarget::Stage(target_id),
            "host-group" => DeploymentTarget::HostGroup(target_id),
            "host" => DeploymentTarget::Host(target_id),
            _ => panic!("Unknown target type: {target_type}"),
        }
    }

    pub fn as_target_type(&self) -> &'static str {
        match self {
            DeploymentTarget::Pipeline(_) => "pipeline",
            DeploymentTarget::Stage(_) => "stage",
            DeploymentTarget::HostGroup(_) => "host-group",
            DeploymentTarget::Host(_) => "host",
        }
    }

    pub fn as_target_id(&self) -> &str {
        match self {
            DeploymentTarget::Pipeline(p) => p,
            DeploymentTarget::Stage(p) => p,
            DeploymentTarget::HostGroup(p) => p,
            DeploymentTarget::Host(p) => p,
        }
    }

    pub fn started_event(&self) -> EventName {
        match self {
            DeploymentTarget::Host(_) => EventName::HostStarted,
            DeploymentTarget::HostGroup(_) => EventName::HostGroupStarted,
            DeploymentTarget::Stage(_) => EventName::StageStarted,
            DeploymentTarget::Pipeline(_) => EventName::PipelineStarted,
        }
    }

    pub fn finished_event(&self) -> EventName {
        match self {
            DeploymentTarget::Host(_) => EventName::HostFinished,
            DeploymentTarget::HostGroup(_) => EventName::HostGroupFinished,
            DeploymentTarget::Stage(_) => EventName::StageFinished,
            DeploymentTarget::Pipeline(_) => EventName::PipelineFinished,
        }
    }
}

impl ToString for DeploymentTarget {
    fn to_string(&self) -> String {
        match self {
            Self::Pipeline(p) => p.to_string(),
            Self::Stage(s) => s.to_string(),
            Self::HostGroup(hg) => hg.to_string(),
            Self::Host(host) => host.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum EventName {
    PipelineStarted,

    StageStarted,
    ArtifactArchitectureRegistered(HostArchitecture),

    HostGroupStarted,

    HostStarted,
    HostArtifactReplicated,
    HostArtifactDeployed,
    HostFinished,

    HostGroupFinished,
    StageFinished,

    PipelineFinished,
}

impl ToString for EventName {
    fn to_string(&self) -> String {
        match self {
            Self::PipelineStarted => "pipeline-started".to_string(),
            Self::StageStarted => "stage-started".to_string(),
            Self::ArtifactArchitectureRegistered(h) => {
                format!("artifact-architecture-registered:{}", h.as_str())
            }
            Self::HostGroupStarted => "host-group-started".to_string(),
            Self::HostStarted => "host-started".to_string(),
            Self::HostArtifactReplicated => "host-artifact-replicated".to_string(),
            Self::HostArtifactDeployed => "host-artifact-deployed".to_string(),
            Self::HostFinished => "host-finished".to_string(),
            Self::HostGroupFinished => "host-group-finished".to_string(),
            Self::StageFinished => "stage-finished".to_string(),
            Self::PipelineFinished => "pipeline-finished".to_string(),
        }
    }
}

impl<E> From<E> for EventName
where
    E: Into<String>,
{
    fn from(value: E) -> Self {
        match value.into().as_str() {
            "pipeline-started" => Self::PipelineStarted,
            "stage-started" => Self::StageStarted,
            "host-group-started" => Self::HostGroupStarted,
            "host-started" => Self::HostStarted,
            "host-artifact-replicated" => Self::HostArtifactReplicated,
            "host-artifact-deployed" => Self::HostArtifactDeployed,
            "host-finished" => Self::HostFinished,
            "host-group-finished" => Self::HostGroupFinished,
            "stage-finished" => Self::StageFinished,
            "pipeline-finished" => Self::PipelineFinished,

            v if v.starts_with("artifact-architecture-registered:") => {
                Self::ArtifactArchitectureRegistered(
                    HostArchitecture::from_str(
                        v.split(":")
                            .last()
                            .expect(format!("Event value {v} must have : delimiter").as_str()),
                    )
                    .expect(
                        format!("Event value {v} could not get host architecture parsed back out")
                            .as_str(),
                    ),
                )
            }

            v => panic!("Invalid EventName: {v}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeploymentEvent(pub String, pub DeploymentTarget, pub EventName);

impl Display for DeploymentEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Deployment{{deployment_id: {}, target: {:?}, event: {:?}}}",
            self.0, self.1, self.2
        ))?;

        Ok(())
    }
}

pub enum PipelineError {
    UnknownStep(DeploymentEvent),
    DatabaseError(anyhow::Error),
}

impl<E> From<E> for PipelineError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        PipelineError::DatabaseError(value.into())
    }
}

pub struct Transition {
    pub next: Vec<DeploymentEvent>,
    // pub previous: DeploymentEvent,
}

pub(crate) async fn is_deployment_complete(
    db: &AntZooStorageClient,
    e: &DeploymentEvent,
) -> Result<bool, anyhow::Error> {
    db.get_deployment(
        &e.0,
        e.1.as_target_type(),
        &e.1.as_target_id(),
        &e.2.to_string(),
    )
    .await
    .map(|r| r.is_some())
}

/// Return all events that come AFTER the `event`, not including the input.
async fn after(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    event: &DeploymentEvent,
) -> Result<Vec<DeploymentEvent>, PipelineError> {
    let mut after: Vec<DeploymentEvent> = Vec::new();
    let mut seen: HashSet<DeploymentEvent> = HashSet::new();
    let mut queue: VecDeque<DeploymentEvent> = VecDeque::new();
    seen.insert(event.clone());
    queue.push_back(event.clone());

    loop {
        if let Some(event) = queue.pop_front() {
            let t = transition(&db, deployment_pipeline_id, &event).await?;

            for next_event in t.next {
                if !seen.contains(&next_event) {
                    queue.push_back(next_event.clone());
                    seen.insert(next_event.clone());
                    after.push(next_event.clone());
                }
            }
        } else {
            break;
        }
    }

    return Ok(after);
}

/// Determine whether a step is DOABLE, based on a few criteria:
///
/// INCOMPLETE(a): the event `a` is incomplete.
/// NEW(a): all events in `after(a)` must be incomplete.
/// READY(a): given a frontier `f`, the steps in `after(f)` do not contain `a`.
///
/// This function returns whether the event is INCOMPLETE, NEW, and READY.
pub async fn is_doable<'a, T: Iterator<Item = &'a DeploymentEvent>>(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    frontier: T,
    event: &DeploymentEvent,
) -> Result<bool, PipelineError> {
    // Check for NEW
    for e in after(db, deployment_pipeline_id, event).await? {
        if is_deployment_complete(db, &e).await? {
            info!("Event {event:?} is not doable, failed NEW: event {e:?} is AFTER, but complete!");
            return Ok(false);
        }
    }

    // Check for READY
    for frontier_event in frontier {
        if after(db, deployment_pipeline_id, frontier_event)
            .await?
            .contains(event)
        {
            info!("Event {event:?} is not doable, failed READY: event {frontier_event:?} has this event in AFTER!");
            return Ok(false);
        }
    }

    Ok(true)
}

/// The set of actions currently "ready to go" for the pipeline.
///
/// Formally, they are the steps `a` such that `next(s) = a`, where `s` is a completed event.
pub async fn frontier(
    db: &AntZooStorageClient,
    revision: &str,
    deployment_pipeline_id: &str,
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

pub async fn transition(
    db: &AntZooStorageClient,
    deployment_pipeline_id: &str,
    event: &DeploymentEvent,
) -> Result<Transition, PipelineError> {
    type T = DeploymentTarget;
    type E = EventName;

    match event {
        DeploymentEvent(id, T::Pipeline(p), E::PipelineStarted) => {
            // The pipeline has begun! Start the first stage.

            let build_stage_id = db
                .get_deployment_pipeline_stage_by_order(p, 0)
                .await?
                .expect("all pipelines should have a stage 0");

            // Find the first stage of the pipeline and start that.
            Ok(Transition {
                next: vec![DeploymentEvent(
                    id.clone(),
                    T::Stage(build_stage_id),
                    E::StageStarted,
                )],
            })
        }

        DeploymentEvent(id, T::Stage(s), E::StageStarted) => {
            // A pipeline stage has begun, start the host group within it
            let stage = db
                .get_deployment_pipeline_stage(s)
                .await?
                .context(event.clone())
                .context("stage should exist if event target")?;

            let next_stages = match stage.3.as_str() {
                "build" => {
                    // The build stage has 3 successors, one for each architecture.

                    db.list_architectures()
                        .await?
                        .into_iter()
                        .map(|arch| {
                            DeploymentEvent(
                                id.clone(),
                                T::Stage(s.clone()),
                                E::ArtifactArchitectureRegistered(arch),
                            )
                        })
                        .collect()
                }

                "deploy" => {
                    let hg = db
                        .get_host_group_by_stage_id(s)
                        .await?
                        .context("deploy stages should have a host group attached")?;

                    vec![DeploymentEvent(
                        id.clone(),
                        T::HostGroup(hg.id),
                        E::HostGroupStarted,
                    )]
                }

                s => {
                    return Err(PipelineError::DatabaseError(anyhow::Error::msg(format!(
                        "Unknown stage type: {s}"
                    ))))
                }
            };

            Ok(Transition { next: next_stages })
        }

        DeploymentEvent(id, T::Stage(s), E::ArtifactArchitectureRegistered(_)) => Ok(Transition {
            next: vec![DeploymentEvent(
                id.clone(),
                T::Stage(s.clone()),
                E::StageFinished,
            )],
        }),

        DeploymentEvent(id, T::HostGroup(hg), E::HostGroupStarted) => {
            // Start deployment to all hosts in the host group, in parallel.
            let hg = db.get_host_group_by_id(hg).await?.context(event.clone())?;

            let next = hg
                .hosts
                .into_iter()
                .map(|host| {
                    DeploymentEvent(
                        id.clone(),
                        DeploymentTarget::Host(host.name),
                        E::HostStarted,
                    )
                })
                .collect::<Vec<DeploymentEvent>>();

            Ok(Transition { next })
        }

        DeploymentEvent(id, T::Host(host), E::HostStarted) => Ok(Transition {
            next: vec![DeploymentEvent(
                id.clone(),
                DeploymentTarget::Host(host.clone()),
                E::HostArtifactReplicated,
            )],
        }),

        DeploymentEvent(id, T::Host(host), E::HostArtifactReplicated) => Ok(Transition {
            next: vec![DeploymentEvent(
                id.clone(),
                DeploymentTarget::Host(host.clone()),
                E::HostArtifactDeployed,
            )],
        }),

        DeploymentEvent(id, T::Host(host), E::HostArtifactDeployed) => Ok(Transition {
            next: vec![DeploymentEvent(
                id.clone(),
                DeploymentTarget::Host(host.clone()),
                event.1.finished_event(),
            )],
        }),

        DeploymentEvent(id, T::Host(host), E::HostFinished) => {
            // Find the host group for the host, given the current revision...
            let host_group = db
                .get_host_group_by_host(deployment_pipeline_id, &host)
                .await
                .context(event.clone())?
                .expect("hosts targeted in a pipeline can be traced back");

            Ok(Transition {
                next: vec![DeploymentEvent(
                    id.clone(),
                    T::HostGroup(host_group),
                    E::HostGroupFinished,
                )],
            })
        }

        DeploymentEvent(id, T::HostGroup(hg), E::HostGroupFinished) => {
            let stage_id = db
                .get_deployment_pipeline_stage_by_host_group(hg)
                .await?
                .context(event.clone())?;

            Ok(Transition {
                next: vec![DeploymentEvent(
                    id.clone(),
                    T::Stage(stage_id),
                    E::StageFinished,
                )],
            })
        }

        DeploymentEvent(id, T::Stage(s), E::StageFinished) => {
            // Start the next stage in the pipeline, if there is one!
            let stage = db
                .get_deployment_pipeline_stage(s)
                .await?
                .context(event.clone())?;

            let next_stage = db
                .get_deployment_pipeline_stage_by_order(&stage.0, stage.2 + 1)
                .await?;

            let next_event = match next_stage {
                None => DeploymentEvent(
                    id.clone(),
                    T::Pipeline(stage.0.clone()),
                    E::PipelineFinished,
                ),
                Some(next_stage) => {
                    DeploymentEvent(id.clone(), T::Stage(next_stage), E::StageStarted)
                }
            };

            Ok(Transition {
                next: vec![next_event],
            })
        }

        DeploymentEvent(_, T::Pipeline(_), E::PipelineFinished) => Ok(Transition { next: vec![] }),

        event => Err(PipelineError::UnknownStep(event.clone())),
    }
}

pub enum JobCompletion<T> {
    Pending,
    Finished(T),
}

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

            let version = state.db.get_revision_version(&revision).await?;

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
