use ant_library::host_architecture::HostArchitecture;
use ant_zoo_storage::HostGroup;
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
    pub name: String,
    pub arch: HostArchitecture,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PipelineHostGroup {
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
pub struct GetPipelineResponse {
    pub project: String,

    pub stages: Vec<PipelineStage>,

    pub events: Vec<PipelineEvent>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEvent {
    pub deployment_id: String,
    pub revision_id: String,
    pub target: DeploymentTarget,
    pub event: EventName,
    pub created_at: DateTime<Utc>,
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

    // Construct the structure of the pipeline

    let mut stages: Vec<PipelineStage> = vec![];

    for (stage_name, stage_id, stage_type) in state
        .db
        .get_deployment_pipeline_stages(&req.project)
        .await?
    {
        match stage_type.as_str() {
            "build" => stages.push(PipelineStage {
                stage_name: stage_name,
                stage_type: PipelineStageType::Build(PipelineBuildStage {}),
            }),
            "deploy" => {
                let hg = state
                    .db
                    .get_host_group_by_stage_id(&stage_id)
                    .await?
                    .context(format!("Stage has no host group: {stage_name} {stage_id}"))?;

                let hg = PipelineHostGroup {
                    name: hg.name,
                    environment: hg.environment,
                    hosts: hg
                        .hosts
                        .into_iter()
                        .map(|h| PipelineHost {
                            name: h.name,
                            arch: h.arch,
                        })
                        .collect(),
                };

                stages.push(PipelineStage {
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

    // Get events applied to the pipeline

    let active_revisions = state
        .db
        .list_revisions_missing_event(&EventName::PipelineFinished.to_string())
        .await?
        .into_iter()
        .filter_map(|(revision, pipe_id)| match *pipe_id == pipeline_id {
            true => Some(revision),
            _ => None,
        })
        .collect::<Vec<String>>();

    let mut all_pipeline_events: Vec<PipelineEvent> = vec![];

    for revision_id in active_revisions {
        let mut pipeline_events = state
            .db
            .list_deployment_events_in_pipeline_revision(&req.project, &revision_id)
            .await?
            .into_iter()
            .map(|e| {
                Ok(PipelineEvent {
                    deployment_id: e.deployment_id,
                    revision_id: e.revision_id,
                    target: match e.target_type.as_str() {
                        "pipeline" => DeploymentTarget::Pipeline(e.target_id),
                        "stage" => DeploymentTarget::Stage(e.target_id),
                        "host-group" => DeploymentTarget::HostGroup(e.target_id),
                        "host" => DeploymentTarget::Host(e.target_id),
                        t => {
                            return Err(AntZookeeperError::InternalServerError(Some(
                                anyhow::Error::msg(format!("Unknown target {t}")),
                            )))
                        }
                    },
                    event: EventName::from(e.event_name),
                    created_at: e.created_at,
                })
            })
            .collect::<Result<Vec<PipelineEvent>, AntZookeeperError>>()?;

        all_pipeline_events.append(&mut pipeline_events);
    }

    Ok((
        StatusCode::OK,
        Json(GetPipelineResponse {
            project: req.project,
            stages: stages,
            events: all_pipeline_events,
        }),
    ))

    // for (stage_name, stage_id, stage_type) in stages {
    //     if stage_type == "build" {
    //         let artifacts = state
    //             .db
    //             .get_latest_artifacts_for_project_for_all_architectures(&req.project)
    //             .await?;

    //         pipeline_stages.push(PipelineStage {
    //             stage_name,
    //             stage_type: PipelineStageType::Build(PipelineBuildStage {
    //                 builds: artifacts
    //                     .into_iter()
    //                     .map(
    //                         |(version, arch, built_at)| -> Result<PipelineBuild, anyhow::Error> {
    //                             Ok(PipelineBuild {
    //                                 architecture: HostArchitecture::from_str(&arch)?,
    //                                 built_version: version,
    //                                 built_at: built_at,
    //                             })
    //                         },
    //                     )
    //                     .collect::<Result<Vec<PipelineBuild>, anyhow::Error>>()?,
    //             }),
    //         });
    //         continue;
    //     }

    //     info!("Building stage {stage_name}...");
    //     let hosts = state.db.get_hosts_in_stage(&stage_id).await?;

    //     let mut pipeline_hosts = vec![];
    //     for host_id in hosts {
    //         info!("Building host {host_id}...");
    //         let history = state.db.get_deployment_history_on_host(&host_id).await?;
    //         let latest = history.first().cloned();

    //         pipeline_hosts.push(PipelineHost {
    //             host_name: host_id,
    //             deployment: latest.map(|l| PipelineHostDeployment {
    //                 deployment_id: l.0,
    //                 deployed_artifact_version: l.1,
    //                 deployed_at: l.2,
    //             }),
    //         });
    //     }

    //     pipeline_stages.push(PipelineStage {
    //         stage_name,
    //         stage_type: PipelineStageType::Deploy(PipelineDeployStage {
    //             hosts: pipeline_hosts,
    //             approved: false,
    //         }),
    //     });
    // }
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
