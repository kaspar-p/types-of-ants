use std::collections::HashMap;

use ant_library::host_architecture::HostArchitecture;
use ant_zookeeper_db::{HostGroup, Revision};
use anyhow::Context;
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

use crate::{
    err::AntZookeeperError,
    event_loop::transition::{DeploymentTarget, EventName},
    state::AntZookeeperState,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPipelineRequest {
    pub project: String,
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
pub struct PipelineBuildStage {}

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
    pub host_group: PipelineHostGroup,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    started_revisions: Vec<RevisionProgress>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    finished_revisions: Vec<RevisionProgress>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    failed_revisions: Vec<FailedRevision>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPipelineResponse {
    pub pipeline_id: String,

    pub project: String,

    pub stages: Vec<PipelineStage>,

    /// The key is a unique identifier
    pub progress: HashMap<String, TargetRevisionProgress>,

    /// The active revisions for a pipeline. All revisions that aren't "pipeline finished",
    /// plus the latest one that IS "pipeline finished"
    pub revisions: Vec<String>,
}

async fn revision_progress(
    state: &AntZookeeperState,
    pipeline_id: &str,
    target: DeploymentTarget,
) -> Result<TargetRevisionProgress, AntZookeeperError> {
    let started = state
        .db
        .list_revisions_with_event(
            pipeline_id,
            target.as_target_type(),
            target.as_target_id(),
            &target.started_event().to_string(),
        )
        .await?;

    let finished = state
        .db
        .list_revisions_with_event(
            pipeline_id,
            target.as_target_type(),
            target.as_target_id(),
            &target.finished_event().to_string(),
        )
        .await?;

    let mut failed_revisions: Vec<FailedRevision> = Vec::new();
    for revision in &started {
        if finished.contains(&revision) {
            continue;
        }

        let failed_jobs: Vec<FailedJob> = state
            .db
            .list_deployment_jobs_after_event(
                &revision.0.id,
                &revision.0.project_id,
                pipeline_id,
                target.as_target_type(),
                target.as_target_id(),
                &target.started_event().to_string(),
            )
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
    if !state.db.get_project(&req.project).await? {
        return Err(AntZookeeperError::ValidationError(format!(
            "No such project: {}",
            req.project
        )));
    }

    let pipeline_id = state
        .db
        .get_deployment_pipeline_by_project(&req.project)
        .await?
        .unwrap();

    let mut progress: HashMap<String, TargetRevisionProgress> = HashMap::new();

    // Insert revision progress of the entire pipeline
    progress.insert(
        pipeline_id.clone(),
        revision_progress(
            &state,
            &pipeline_id,
            DeploymentTarget::Pipeline(pipeline_id.clone()),
        )
        .await?,
    );

    let mut stages: Vec<PipelineStage> = vec![];

    for (stage_name, stage_id, stage_type) in state
        .db
        .get_deployment_pipeline_stages(&req.project)
        .await?
    {
        let stage_target = DeploymentTarget::Stage(stage_id.clone());
        progress.insert(
            stage_id.clone(),
            revision_progress(&state, &pipeline_id, stage_target).await?,
        );

        match stage_type.as_str() {
            "build" => stages.push(PipelineStage {
                stage_id,
                stage_name: stage_name,
                stage_type: PipelineStageType::Build(PipelineBuildStage {}),
            }),
            "deploy" => {
                let hg = state
                    .db
                    .get_host_group_by_stage_id(&stage_id)
                    .await?
                    .context(format!("Stage has no host group: {stage_name} {stage_id}"))?;

                // Insert revision progress for host group
                {
                    let hg_target = DeploymentTarget::HostGroup(hg.id.clone());
                    progress.insert(
                        hg.id.clone(),
                        revision_progress(&state, &pipeline_id, hg_target).await?,
                    );
                }

                // Insert revision progress for each host
                for host in &hg.hosts {
                    let host_target = DeploymentTarget::Host(host.name.clone());
                    progress.insert(
                        host.name.clone(),
                        revision_progress(&state, &pipeline_id, host_target).await?,
                    );
                }

                let hg = PipelineHostGroup {
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

                stages.push(PipelineStage {
                    stage_id,
                    stage_name,
                    stage_type: PipelineStageType::Deploy(PipelineDeployStage { host_group: hg }),
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
        let mut revisions = state
            .db
            .list_revisions_missing_event(&EventName::PipelineFinished.to_string())
            .await?
            .into_iter()
            .filter_map(|(rev, pipeline)| match *pipeline == pipeline_id {
                true => Some(rev),
                false => None,
            })
            .collect::<Vec<Revision>>();

        let target = DeploymentTarget::Pipeline(pipeline_id.clone());
        let mut latest_finished = state
            .db
            .list_revisions_with_event(
                &pipeline_id,
                target.as_target_type(),
                target.as_target_id(),
                &target.finished_event().to_string(),
            )
            .await?;
        latest_finished.sort_by(|a, b| a.0.seq.cmp(&b.0.seq));

        if let Some(latest) = latest_finished.pop() {
            revisions.push(latest.0);
        }

        revisions
    };

    Ok((
        StatusCode::OK,
        Json(GetPipelineResponse {
            pipeline_id,
            project: req.project,
            stages,
            progress,
            revisions: revisions.into_iter().map(|r| r.id).collect(),
        }),
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutPipelineStage {
    pub name: String,
    pub host_group_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutPipelineRequest {
    pub project: String,
    pub stages: Vec<PutPipelineStage>,
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
    let pipeline_id = state
        .db
        .get_deployment_pipeline_by_project(&req.project)
        .await?;
    if pipeline_id.is_none() {
        return Err(AntZookeeperError::ValidationError(format!(
            "No pipeline for project: {}",
            req.project
        )));
    }
    let pipeline_id = pipeline_id.unwrap();

    for stage in &req.stages {
        let group = state.db.get_host_group_by_id(&stage.host_group_id).await?;

        match group {
            None => {
                return Err(AntZookeeperError::ValidationError(format!(
                    "No such host group: {}",
                    stage.host_group_id
                )));
            }
            Some(group) if group.hosts.is_empty() => {
                return Err(AntZookeeperError::ValidationError(format!(
                    "Host group {} cannot be added to a pipeline because it has no hosts.",
                    stage.host_group_id
                )));
            }
            _ => (),
        }
    }

    let stages = state
        .db
        .get_deployment_pipeline_stages(&req.project)
        .await?;

    // delete previous pipeline definition
    for (i, (stage_name, stage_id, _)) in stages.iter().enumerate() {
        if i == 0 {
            continue; // build stage can't be deleted
        }

        info!("Deleting stage {stage_name} ({stage_id})");
        state.db.delete_deployment_pipeline_stage(&stage_id).await?;
    }

    // create new pipeline definition
    for (i, stage) in req.stages.iter().enumerate() {
        let id = state
            .db
            .create_deployment_pipeline_deployment_stage(
                &pipeline_id,
                &stage.name,
                &stage.host_group_id,
                i as i32 + 1, // build stage is always 0
            )
            .await?;
        info!(
            "Created deployment stage {} (id {}) with host group {}",
            stage.name, id, stage.host_group_id
        );
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
        .create_host_group(&req.name, &req.environment)
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
