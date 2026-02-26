use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
};

use ant_library::host_architecture::HostArchitecture;
use ant_zookeeper_db::AntZooStorageClient;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::info;

// #[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, PartialOrd, Ord)]
// #[serde(rename_all = "camelCase")]
// pub enum DeploymentTarget {
//     Pipeline(String),
//     Stage(String),
//     HostGroup(String),
//     Host(String),
// }

// impl DeploymentTarget {
//     pub fn from_strings(target_type: &str, target_id: String) -> DeploymentTarget {
//         match target_type {
//             "pipeline" => DeploymentTarget::Pipeline(target_id),
//             "stage" => DeploymentTarget::Stage(target_id),
//             "host-group" => DeploymentTarget::HostGroup(target_id),
//             "host" => DeploymentTarget::Host(target_id),
//             _ => panic!("Unknown target type: {target_type}"),
//         }
//     }

//     pub fn as_target_type(&self) -> &'static str {
//         match self {
//             DeploymentTarget::Pipeline(_) => "pipeline",
//             DeploymentTarget::Stage(_) => "stage",
//             DeploymentTarget::HostGroup(_) => "host-group",
//             DeploymentTarget::Host(_) => "host",
//         }
//     }

//     pub fn as_target_id(&self) -> &str {
//         match self {
//             DeploymentTarget::Pipeline(p) => p,
//             DeploymentTarget::Stage(p) => p,
//             DeploymentTarget::HostGroup(p) => p,
//             DeploymentTarget::Host(p) => p,
//         }
//     }

//     pub fn started_event(&self) -> Event {
//         match self {
//             DeploymentTarget::Host(_) => Event::HostStarted {
//                 project: "".to_string(),
//             },
//             DeploymentTarget::HostGroup(_) => Event::HostGroupStarted,
//             DeploymentTarget::Stage(_) => Event::StageStarted,
//             DeploymentTarget::Pipeline(_) => Event::PipelineStarted,
//         }
//     }

//     pub fn finished_event(&self) -> Event {
//         match self {
//             DeploymentTarget::Host(_) => Event::HostFinished,
//             DeploymentTarget::HostGroup(_) => Event::HostGroupFinished,
//             DeploymentTarget::Stage(_) => Event::StageFinished,
//             DeploymentTarget::Pipeline(_) => Event::PipelineFinished,
//         }
//     }
// }

// impl ToString for DeploymentTarget {
//     fn to_string(&self) -> String {
//         match self {
//             Self::Pipeline(p) => p.to_string(),
//             Self::Stage(s) => s.to_string(),
//             Self::HostGroup(hg) => hg.to_string(),
//             Self::Host(host) => host.to_string(),
//         }
//     }
// }

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Event {
    PipelineStarted {
        pipeline_id: String,
    },
    PipelineFinished {
        pipeline_id: String,
    },

    StageStarted {
        stage_id: String,
    },
    ArtifactRegistered {
        stage_id: String,
        arch: HostArchitecture,
    },
    StageFinished {
        stage_id: String,
    },

    HostGroupStarted {
        host_group_id: String,
    },
    HostGroupFinished {
        host_group_id: String,
    },

    HostStarted {
        host_group_id: String,
        host: String,
    },
    HostArtifactReplicated {
        host_group_id: String,
        host: String,
    },
    HostArtifactDeployed {
        host_group_id: String,
        host: String,
    },
    HostFinished {
        host_group_id: String,
        host: String,
    },
}

impl ToString for Event {
    fn to_string(&self) -> String {
        serde_json::to_string(&self)
            .expect(&format!("Event failed to serialize to string: {self:?}"))
    }
}

impl<E> From<E> for Event
where
    E: Into<String>,
{
    fn from(value: E) -> Self {
        let str = value.into();
        serde_json::from_str(&str)
            .expect(&format!("Event failed to deserialize from string: {str}"))
    }
}

/// Represents (revision_id, event)
#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct DeploymentEvent(pub String, pub Event);

impl DeploymentEvent {
    pub fn for_other_revision(&self, rev: String) -> Self {
        DeploymentEvent(rev, self.1.clone())
    }
}

impl Display for DeploymentEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Deployment{{revision: {}, event: {:?}}}",
            self.0, self.1
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
    db.get_deployment(&e.0, &e.1.to_string())
        .await
        .map(|r| r.is_some())
}

/// Return all events that come AFTER the `event`, not including the input.
pub(crate) async fn after(
    db: &AntZooStorageClient,
    event: &DeploymentEvent,
) -> Result<Vec<DeploymentEvent>, PipelineError> {
    let mut after: Vec<DeploymentEvent> = Vec::new();
    let mut seen: HashSet<DeploymentEvent> = HashSet::new();
    let mut queue: VecDeque<DeploymentEvent> = VecDeque::new();
    seen.insert(event.clone());
    queue.push_back(event.clone());

    loop {
        if let Some(event) = queue.pop_front() {
            let t = transition(&db, &event).await?;

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
/// NEW(a): all events in `after(a)` must be incomplete.
///     This property ensures that adding hosts to host groups does not deploy to that host.
///
/// READY(a): given a frontier `f`, the steps in `after(f)` do not contain `a`.
///     This property ensures that "collapsing" nodes in the graph (nodes that have multiple
///     nodes as predecessors) do not begin too early. They must begin only if ALL of their
///     predecessors are finished, not just the first one.
///
/// This function returns whether the event is NEW and READY.
pub async fn is_doable<'a, T: Iterator<Item = &'a DeploymentEvent>>(
    db: &AntZooStorageClient,
    frontier: T,
    event: &DeploymentEvent,
) -> Result<bool, PipelineError> {
    // Check for NEW
    for e in after(db, event).await? {
        if is_deployment_complete(db, &e).await? {
            info!("Event {event:?} is not doable, failed NEW: event {e:?} is AFTER, but complete!");
            return Ok(false);
        }
    }

    // Check for READY
    for frontier_event in frontier {
        if after(db, frontier_event).await?.contains(event) {
            info!("Event {event:?} is not doable, failed READY: event {frontier_event:?} has this event in AFTER!");
            return Ok(false);
        }
    }

    Ok(true)
}

pub async fn transition(
    db: &AntZooStorageClient,
    event: &DeploymentEvent,
) -> Result<Transition, PipelineError> {
    type E = Event;

    match event {
        DeploymentEvent(rev, E::PipelineStarted { pipeline_id }) => {
            // The pipeline has begun! Start the first stages, those with no dependencies
            let initial_stages = db
                .list_deployment_stages_with_no_previous_adjacencies(pipeline_id)
                .await?
                .into_iter()
                .map(|stage_id| DeploymentEvent(rev.clone(), E::StageStarted { stage_id }))
                .collect();

            // Find the first stage of the pipeline and start that.
            Ok(Transition {
                next: initial_stages,
            })
        }

        DeploymentEvent(rev, E::StageStarted { stage_id }) => {
            // A pipeline stage has begun, start the host group within it
            let stage = db
                .get_deployment_pipeline_stage(stage_id)
                .await?
                .context(event.clone())
                .context("stage should exist if event target")?;

            let next_events = match stage.2.as_str() {
                "build" => {
                    // The build stage has 3 successors, one for each architecture.
                    db.list_architectures()
                        .await?
                        .into_iter()
                        .map(|arch| {
                            DeploymentEvent(
                                rev.clone(),
                                E::ArtifactRegistered {
                                    stage_id: stage_id.clone(),
                                    arch,
                                },
                            )
                        })
                        .collect()
                }

                "deploy" => {
                    let hgs = db.get_host_groups_by_stage_id(stage_id).await?;

                    assert_ne!(hgs.len(), 0);

                    // Deploy to all host groups in parallel.
                    hgs.into_iter()
                        .map(|hg| {
                            DeploymentEvent(
                                rev.clone(),
                                E::HostGroupStarted {
                                    host_group_id: hg.id,
                                },
                            )
                        })
                        .collect()
                }

                s => {
                    return Err(PipelineError::DatabaseError(anyhow::Error::msg(format!(
                        "Unknown stage type: {s}"
                    ))))
                }
            };

            Ok(Transition { next: next_events })
        }

        DeploymentEvent(rev, E::ArtifactRegistered { stage_id, .. }) => Ok(Transition {
            next: vec![DeploymentEvent(
                rev.clone(),
                E::StageFinished {
                    stage_id: stage_id.clone(),
                },
            )],
        }),

        DeploymentEvent(rev, E::HostGroupStarted { host_group_id }) => {
            // Start deployment to all hosts in the host group, in parallel.
            let hg = db
                .get_host_group_by_id(host_group_id)
                .await?
                .context(event.clone())?;

            let next = hg
                .hosts
                .into_iter()
                .map(|host| {
                    DeploymentEvent(
                        rev.clone(),
                        E::HostStarted {
                            host_group_id: host_group_id.clone(),
                            host: host.name,
                        },
                    )
                })
                .collect::<Vec<DeploymentEvent>>();

            Ok(Transition { next })
        }

        DeploymentEvent(
            rev,
            E::HostStarted {
                host_group_id,
                host,
            },
        ) => Ok(Transition {
            next: vec![DeploymentEvent(
                rev.clone(),
                E::HostArtifactReplicated {
                    host: host.clone(),
                    host_group_id: host_group_id.clone(),
                },
            )],
        }),

        DeploymentEvent(
            rev,
            E::HostArtifactReplicated {
                host_group_id,
                host,
            },
        ) => Ok(Transition {
            next: vec![DeploymentEvent(
                rev.clone(),
                E::HostArtifactDeployed {
                    host_group_id: host_group_id.clone(),
                    host: host.clone(),
                },
            )],
        }),

        DeploymentEvent(
            rev,
            E::HostArtifactDeployed {
                host_group_id,
                host,
            },
        ) => Ok(Transition {
            next: vec![DeploymentEvent(
                rev.clone(),
                E::HostFinished {
                    host_group_id: host_group_id.clone(),
                    host: host.clone(),
                },
            )],
        }),

        DeploymentEvent(rev, E::HostFinished { host_group_id, .. }) => Ok(Transition {
            next: vec![DeploymentEvent(
                rev.clone(),
                E::HostGroupFinished {
                    host_group_id: host_group_id.clone(),
                },
            )],
        }),

        DeploymentEvent(rev, E::HostGroupFinished { host_group_id }) => {
            let stage_id = db
                .get_deployment_pipeline_stage_by_host_group(host_group_id)
                .await?
                .context(event.clone())?;

            Ok(Transition {
                next: vec![DeploymentEvent(
                    rev.clone(),
                    E::StageFinished {
                        stage_id: stage_id.clone(),
                    },
                )],
            })
        }

        DeploymentEvent(rev, E::StageFinished { stage_id }) => {
            // Start the next stage in the pipeline, if there is one!
            let stage = db
                .get_deployment_pipeline_stage(stage_id)
                .await?
                .context(event.clone())?;

            let next_stages: Vec<DeploymentEvent> = db
                .list_deployment_pipeline_stages_after(&stage_id)
                .await?
                .into_iter()
                .map(|next_stage_id| {
                    DeploymentEvent(
                        rev.clone(),
                        E::StageStarted {
                            stage_id: next_stage_id,
                        },
                    )
                })
                .collect();

            // Finish the pipeline if there aren't any stages left
            let next_events = match next_stages.len() {
                0 => vec![DeploymentEvent(
                    rev.clone(),
                    E::PipelineFinished {
                        pipeline_id: stage.0,
                    },
                )],
                _ => next_stages,
            };

            Ok(Transition { next: next_events })
        }

        DeploymentEvent(_, E::PipelineFinished { .. }) => Ok(Transition { next: vec![] }),
    }
}
