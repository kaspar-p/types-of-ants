use std::collections::{HashMap, HashSet};

use ant_library::host_architecture::HostArchitecture;
use ant_zookeeper_db::HostGroup;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{err::AntZookeeperError, event_loop::transition::Event, state::AntZookeeperState};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPipelineRequest {
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    pub stage_id: String,
    pub stage_name: String,
    pub stage_type: PipelineStageType,
}

impl PipelineStage {
    pub fn build_stage(self) -> PipelineBuildStage {
        match self.stage_type {
            PipelineStageType::Build(s) => s,
            _ => panic!("Not a build stage!"),
        }
    }

    pub fn deploy_stage(self) -> PipelineDeployStage {
        match self.stage_type {
            PipelineStageType::Deploy(s) => s,
            _ => panic!("Not a deploy stage!"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum PipelineStageType {
    Build(PipelineBuildStage),
    Deploy(PipelineDeployStage),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineBuildStage {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineHost {
    pub host_id: String,

    pub name: String,
    pub arch: HostArchitecture,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineHostGroup {
    pub host_group_id: String,

    pub name: String,
    pub environment: String,
    pub hosts: Vec<PipelineHost>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDeployStage {
    pub host_groups: Vec<PipelineHostGroup>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionProgress {
    revision: String,
    reached_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedJob {
    job_id: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FailedRevision {
    revision: String,
    failed_jobs: Vec<FailedJob>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRevisionProgress {
    started_revisions: Vec<RevisionProgress>,
    finished_revisions: Vec<RevisionProgress>,
    failed_revisions: Vec<FailedRevision>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPipelineResponse {
    pub pipeline_id: String,

    pub name: String,

    /// Each entry in the vector is a parallel list of stages
    pub stages: Vec<Vec<PipelineStage>>,

    /// The key is a unique identifier
    pub progress: HashMap<String, TargetRevisionProgress>,

    /// The active revisions for a pipeline. All revisions that aren't "pipeline finished",
    /// plus the latest one that IS "pipeline finished"
    pub revisions: Vec<String>,
}

async fn revision_progress(
    state: &AntZookeeperState,
    starting_event: Event,
    ending_event: Event,
) -> Result<TargetRevisionProgress, AntZookeeperError> {
    let started = state
        .db
        .list_revisions_with_event(&starting_event.to_string())
        .await?;

    let finished = state
        .db
        .list_revisions_with_event(&ending_event.to_string())
        .await?;

    let mut failed_revisions: Vec<FailedRevision> = Vec::new();
    for revision in &started {
        if finished.contains(&revision) {
            continue;
        }

        let failed_jobs: Vec<FailedJob> = state
            .db
            .list_deployment_jobs_after_event(&revision.0.id, &starting_event.to_string())
            .await?
            .into_iter()
            .filter(|(_, _, is_success, _, _)| !*is_success)
            .map(|(job_id, _, _, started_at, finished_at)| FailedJob {
                job_id,
                started_at,
                finished_at,
            })
            .collect();

        if failed_jobs.len() > 0 {
            failed_revisions.push(FailedRevision {
                revision: revision.0.id.clone(),
                failed_jobs,
            });
        }
    }

    Ok(TargetRevisionProgress {
        started_revisions: started
            .into_iter()
            .map(|rev| RevisionProgress {
                revision: rev.0.id,
                reached_at: rev.1,
            })
            .collect(),
        finished_revisions: finished
            .into_iter()
            .map(|rev| RevisionProgress {
                revision: rev.0.id,
                reached_at: rev.1,
            })
            .collect(),
        failed_revisions,
    })
}

async fn get_pipeline(
    State(state): State<AntZookeeperState>,
    Query(req): Query<GetPipelineRequest>,
) -> Result<(StatusCode, Json<GetPipelineResponse>), AntZookeeperError> {
    let pipeline_id = match state.db.get_deployment_pipeline_by_name(&req.name).await? {
        None => {
            return Err(AntZookeeperError::ValidationError(format!(
                "No such pipeline: {}",
                req.name
            )));
        }
        Some(pipeline_id) => pipeline_id,
    };

    let mut progress: HashMap<String, TargetRevisionProgress> = HashMap::new();

    // Insert revision progress of the entire pipeline
    progress.insert(
        pipeline_id.clone(),
        revision_progress(
            &state,
            Event::PipelineStarted {
                pipeline_id: pipeline_id.clone(),
            },
            Event::PipelineFinished {
                pipeline_id: pipeline_id.clone(),
            },
        )
        .await?,
    );

    let mut stages: Vec<PipelineStage> = vec![];

    for (stage_name, stage_id, stage_type, project_id) in state
        .db
        .list_deployment_pipeline_stages(&pipeline_id)
        .await?
    {
        progress.insert(
            stage_id.clone(),
            revision_progress(
                &state,
                Event::StageStarted {
                    stage_id: stage_id.clone(),
                },
                Event::StageFinished {
                    stage_id: stage_id.clone(),
                },
            )
            .await?,
        );

        match stage_type.as_str() {
            "build" => stages.push(PipelineStage {
                stage_id: stage_id.clone(),
                stage_name: stage_name,
                stage_type: PipelineStageType::Build(PipelineBuildStage {
                    project: project_id
                        .expect(&format!("build stage {} should have project", stage_id)),
                }),
            }),
            "deploy" => {
                let hgs = state.db.get_host_groups_by_stage_id(&stage_id).await?;

                let mut pipeline_hgs = vec![];
                for hg in hgs {
                    // Insert revision progress for host group
                    {
                        progress.insert(
                            hg.id.clone(),
                            revision_progress(
                                &state,
                                Event::HostGroupStarted {
                                    host_group_id: hg.id.clone(),
                                },
                                Event::HostGroupFinished {
                                    host_group_id: hg.id.clone(),
                                },
                            )
                            .await?,
                        );
                    }

                    // Insert revision progress for each host
                    for host in &hg.hosts {
                        progress.insert(
                            format!("{}#{}", hg.id, host.name),
                            revision_progress(
                                &state,
                                Event::HostStarted {
                                    host_group_id: hg.id.clone(),
                                    host: host.name.clone(),
                                },
                                Event::HostFinished {
                                    host_group_id: hg.id.clone(),
                                    host: host.name.clone(),
                                },
                            )
                            .await?,
                        );
                    }

                    let pipeline_hg = PipelineHostGroup {
                        host_group_id: hg.id,
                        name: hg.name,
                        environment: hg.environment,
                        hosts: hg
                            .hosts
                            .into_iter()
                            .map(|h| PipelineHost {
                                host_id: h.name.clone(),
                                name: h.name,
                                arch: h.arch,
                            })
                            .collect(),
                    };

                    pipeline_hgs.push(pipeline_hg);
                }

                stages.push(PipelineStage {
                    stage_id,
                    stage_name,
                    stage_type: PipelineStageType::Deploy(PipelineDeployStage {
                        host_groups: pipeline_hgs,
                    }),
                })
            }
            t => {
                return Err(AntZookeeperError::InternalServerError(Some(
                    anyhow::Error::msg(format!("Unknown stage format: {t}")),
                )))
            }
        }
    }

    let revisions = {
        let event = &Event::PipelineFinished {
            pipeline_id: pipeline_id.clone(),
        }
        .to_string();

        let mut revisions = state
            .db
            .list_revisions_with_event(
                &Event::PipelineStarted {
                    pipeline_id: pipeline_id.clone(),
                }
                .to_string(),
            )
            .await?;

        let not_ended_revisions = state.db.list_revisions_missing_event(event).await?;

        // Only take revisions that have STARTED but not ENDED
        revisions = revisions
            .into_iter()
            .filter(|r| not_ended_revisions.contains(&r.0))
            .collect();

        // Plus the latest one that ENDED
        let mut latest_finished = state.db.list_revisions_with_event(event).await?;
        latest_finished.sort_by(|a, b| a.0.seq.cmp(&b.0.seq));

        if let Some(latest) = latest_finished.pop() {
            revisions.push(latest);
        }

        revisions
    };

    let initial = state
        .db
        .list_deployment_stages_with_no_previous_adjacencies(&pipeline_id)
        .await?;

    let mut phases: Vec<Vec<String>> = vec![initial];
    let mut i: usize = 0;
    loop {
        // For each phase, get all stages in the NEXT set of that phase and add to the next phase
        let mut next_phase: HashSet<String> = HashSet::new();
        match phases.get(i) {
            None => break,
            Some(phase) => {
                for stage_id in phase {
                    let next = state
                        .db
                        .list_deployment_pipeline_stages_after(&stage_id)
                        .await?;

                    next_phase.extend(next);
                }
            }
        }

        if !next_phase.is_empty() {
            phases.push(next_phase.into_iter().collect());
        }
        next_phase = HashSet::new();
        i += 1;
    }

    Ok((
        StatusCode::OK,
        Json(GetPipelineResponse {
            pipeline_id,
            name: req.name,
            stages: phases
                .into_iter()
                .map(|phase_stages| {
                    phase_stages
                        .into_iter()
                        .map(|stage_id| {
                            stages
                                .iter()
                                .find(|stage| stage.stage_id == stage_id)
                                .unwrap()
                                .clone()
                        })
                        .collect()
                })
                .collect(),
            progress,
            revisions: revisions.into_iter().map(|r| r.0.id).collect(),
        }),
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutPipelineStage {
    pub name: String,
    pub host_group_ids: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutPipelineRequest {
    pub name: String,
    /// First nesting is the PHASING of the stages, e.g. phase 0 happens first, ...
    /// and then it's stages that happen in parallel within that phase.
    pub stages: Vec<Vec<PutPipelineStage>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutPipelineResponse {
    pub created_at: DateTime<Utc>,
}

async fn put_pipeline(
    State(state): State<AntZookeeperState>,
    Json(req): Json<PutPipelineRequest>,
) -> Result<(StatusCode, Json<PutPipelineResponse>), AntZookeeperError> {
    let pipeline_id = match state.db.get_deployment_pipeline_by_name(&req.name).await? {
        None => state.db.create_deployment_pipeline(&req.name).await?,
        Some(pipeline_id) => pipeline_id,
    };

    for phase in &req.stages {
        for stage in phase {
            if stage.host_group_ids.is_empty() {
                return Err(AntZookeeperError::ValidationError(format!(
                    "Stage must include at least one host group: {}",
                    &stage.name
                )));
            }

            if stage.host_group_ids.len() > 10 {
                return Err(AntZookeeperError::ValidationError(format!(
                    "Stage has too many host groups: {}",
                    &stage.name
                )));
            }

            for host_group_id in &stage.host_group_ids {
                let group = state.db.get_host_group_by_id(&host_group_id).await?;

                match group {
                    None => {
                        return Err(AntZookeeperError::ValidationError(format!(
                            "No such host group: {}",
                            host_group_id
                        )));
                    }
                    Some(group) if group.hosts.is_empty() => {
                        return Err(AntZookeeperError::ValidationError(format!(
                            "Host group {} cannot be added to a pipeline because it has no hosts.",
                            host_group_id
                        )));
                    }
                    _ => (),
                }
            }
        }
    }

    let stages = state
        .db
        .list_deployment_pipeline_stages(&pipeline_id)
        .await?;

    // delete previous pipeline definition
    for (stage_name, stage_id, _, _) in stages {
        info!("Deleting stage {stage_name} ({stage_id})");
        state.db.delete_deployment_pipeline_stage(&stage_id).await?;
    }

    // create new pipeline definition

    // Inject parallel build stages for each project listed in the host groups deployed later.
    let mut projects = HashSet::new();
    for phase in &req.stages {
        for stage in phase {
            for host_group_id in &stage.host_group_ids {
                let hg = state
                    .db
                    .get_host_group_by_id(&host_group_id)
                    .await?
                    .unwrap();
                projects.insert(hg.project);
            }
        }
    }

    // Create phase of parallel build stages
    let mut previous_phase_stage_ids: Vec<String> = vec![];
    for project in projects {
        let id = state
            .db
            .create_deployment_pipeline_deployment_stage(
                &pipeline_id,
                &format!("build:{}", project),
                "build",
                None,
                Some(&project),
                &vec![],
            )
            .await?;
        previous_phase_stage_ids.push(id);
    }

    for (i, phase) in req.stages.iter().enumerate() {
        let mut phase_stage_ids = vec![];
        for stage in phase {
            let id = state
                .db
                .create_deployment_pipeline_deployment_stage(
                    &pipeline_id,
                    &stage.name,
                    "deploy",
                    Some(&stage.host_group_ids),
                    None,
                    &previous_phase_stage_ids,
                )
                .await?;
            info!(
                "Created deployment stage {} (phase={i}) (id {}) with host groups: {}",
                &stage.name,
                id,
                &stage.host_group_ids.join(", ")
            );

            phase_stage_ids.push(id);
        }

        previous_phase_stage_ids = phase_stage_ids;
    }

    Ok((
        StatusCode::OK,
        Json(PutPipelineResponse {
            created_at: Utc::now(),
        }),
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHostGroupRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHostGroupResponse {
    pub host_group: HostGroup,
}

async fn get_host_group(
    State(state): State<AntZookeeperState>,
    Query(req): Query<GetHostGroupRequest>,
) -> Result<(StatusCode, Json<GetHostGroupResponse>), AntZookeeperError> {
    match state.db.get_host_group_by_name(&req.name).await? {
        None => {
            return Err(AntZookeeperError::ValidationError(format!(
                "No host group named: {}",
                req.name
            )))
        }
        Some(group) => {
            return Ok((
                StatusCode::OK,
                Json(GetHostGroupResponse { host_group: group }),
            ))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHostGroupRequest {
    pub name: String,
    pub project: String,
    pub environment: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHostGroupResponse {
    pub name: String,
    pub id: String,
}

async fn create_host_group(
    State(state): State<AntZookeeperState>,
    Json(req): Json<CreateHostGroupRequest>,
) -> Result<(StatusCode, Json<CreateHostGroupResponse>), AntZookeeperError> {
    if state.db.get_host_group_by_name(&req.name).await?.is_some() {
        return Err(AntZookeeperError::validation_msg(
            "Host group with that name already exists.",
        ));
    }

    if !state.db.get_project(&req.project).await? {
        return Err(AntZookeeperError::validation_msg(&format!(
            "No such project: {}",
            req.project
        )));
    }

    match req.environment.as_str() {
        "dev" | "beta" | "prod" => {}
        _ => {
            return Err(AntZookeeperError::validation_msg(
                "Environment must be 'dev', 'beta', or 'prod'.",
            ));
        }
    };

    let id = state
        .db
        .create_host_group(&req.name, &req.project, &req.environment)
        .await?;

    Ok((
        StatusCode::OK,
        Json(CreateHostGroupResponse { name: req.name, id }),
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddHostToHostGroupRequest {
    pub host_group_id: String,
    pub host_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddHostToHostGroupResponse {
    host_already_in_host_group: bool,
}

async fn add_host_to_host_group(
    State(state): State<AntZookeeperState>,
    Json(req): Json<AddHostToHostGroupRequest>,
) -> Result<(StatusCode, Json<AddHostToHostGroupResponse>), AntZookeeperError> {
    if !state.db.host_group_exists_by_id(&req.host_group_id).await? {
        return Err(AntZookeeperError::validation_msg("No such host group."));
    }

    if state.db.get_host(&req.host_id).await?.is_none() {
        return Err(AntZookeeperError::validation_msg("No such host."));
    }

    if state
        .db
        .host_in_host_group(&req.host_group_id, &req.host_id)
        .await?
    {
        info!("Host already in host group...");
        return Ok((
            StatusCode::OK,
            Json(AddHostToHostGroupResponse {
                host_already_in_host_group: true,
            }),
        ));
    }

    info!("Adding host to group...");

    state
        .db
        .add_host_to_host_group(&req.host_group_id, &req.host_id)
        .await?;

    Ok((
        StatusCode::OK,
        Json(AddHostToHostGroupResponse {
            host_already_in_host_group: false,
        }),
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveHostFromHostGroupRequest {
    pub host_group_id: String,
    pub host_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveHostFromHostGroupResponse {
    pub host_was_present: bool,
}

async fn remove_host_from_host_group(
    State(state): State<AntZookeeperState>,
    Json(req): Json<RemoveHostFromHostGroupRequest>,
) -> Result<(StatusCode, Json<RemoveHostFromHostGroupResponse>), AntZookeeperError> {
    if !state.db.host_group_exists_by_id(&req.host_group_id).await? {
        return Err(AntZookeeperError::validation_msg("No such host group."));
    }

    if state.db.get_host(&req.host_id).await?.is_none() {
        return Err(AntZookeeperError::validation_msg("No such host."));
    }

    let host_was_present = state
        .db
        .remove_host_from_host_group(&req.host_group_id, &req.host_id)
        .await?;

    Ok((
        StatusCode::OK,
        Json(RemoveHostFromHostGroupResponse { host_was_present }),
    ))
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new()
        .route_with_tsr(
            "/host-group/host-group",
            get(get_host_group).post(create_host_group),
        )
        .route_with_tsr(
            "/host-group/host",
            post(add_host_to_host_group).delete(remove_host_from_host_group),
        )
        .route_with_tsr("/pipeline", get(get_pipeline).post(put_pipeline))
}
