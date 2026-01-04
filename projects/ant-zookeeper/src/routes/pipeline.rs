use ant_zoo_storage::HostGroup;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{err::AntZookeeperError, state::AntZookeeperState};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPipelineRequest {
    pub project: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineHost {
    pub host_name: String,
    pub deployed_artifact_version: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    pub stage_name: String,
    pub hosts: Vec<PipelineHost>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPipelineResponse {
    pub project: String,
    pub stages: Vec<PipelineStage>,
}

#[debug_handler]
async fn get_pipeline(
    State(state): State<AntZookeeperState>,
    Json(req): Json<GetPipelineRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.get_project(&req.project).await? {
        return Err(AntZookeeperError::ValidationError {
            msg: format!("No such project: {}", req.project),
            e: None,
        });
    }

    let stages = state
        .db
        .get_deployment_pipeline_stages(&req.project)
        .await?;

    let mut pipeline_stages: Vec<PipelineStage> = vec![];

    for (stage_name, stage_id) in stages {
        info!("Building stage {stage_name}...");
        let hosts = state.db.get_hosts_in_stage(&stage_id).await?;

        let mut pipeline_hosts = vec![];
        for host_id in hosts {
            info!("Building host {host_id}...");
            let history = state.db.get_deployment_history_on_host(&host_id).await?;
            let latest = history.first().cloned();

            pipeline_hosts.push(PipelineHost {
                host_name: host_id,
                deployed_artifact_version: latest.clone().map(|l| l.0),
                deployed_at: latest.map(|l| l.1),
            });
        }

        pipeline_stages.push(PipelineStage {
            stage_name,
            hosts: pipeline_hosts,
        });
    }

    Ok((
        StatusCode::OK,
        Json(GetPipelineResponse {
            project: req.project,
            stages: pipeline_stages,
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

async fn put_pipeline(
    State(state): State<AntZookeeperState>,
    Json(req): Json<PutPipelineRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    let pipeline_id = state
        .db
        .deployment_pipeline_exists_by_project(&req.project)
        .await?;

    for stage in &req.stages {
        let group = state.db.get_host_group_by_id(&stage.host_group_id).await?;

        match group {
            None => {
                return Err(AntZookeeperError::validation_msg(&format!(
                    "No such host group: {}",
                    stage.host_group_id
                )));
            }
            Some(group) if group.hosts.is_empty() => {
                return Err(AntZookeeperError::validation_msg(&format!(
                    "Host group {} cannot be added to a pipeline because it has no hosts.",
                    stage.host_group_id
                )));
            }
            _ => (),
        }
    }

    if pipeline_id.is_none() {
        return Err(AntZookeeperError::validation_msg(
            "No such deployment pipeline.",
        ));
    }
    let pipeline_id = pipeline_id.unwrap();

    let stages = state
        .db
        .get_deployment_pipeline_stages(&req.project)
        .await?;

    // delete previous pipeline definition
    for (stage_name, stage_id) in stages {
        info!("Deleting stage {stage_name} ({stage_id})");
        state.db.delete_deployment_pipeline_stage(&stage_id).await?;
    }

    // create new pipeline definition
    for (i, stage) in req.stages.iter().enumerate() {
        info!(
            "Creating stage {} with host group {}",
            stage.name, stage.host_group_id
        );
        state
            .db
            .create_deployment_pipeline_stage(
                &pipeline_id,
                &stage.name,
                &stage.host_group_id,
                i as i32,
            )
            .await?;
    }

    Ok(StatusCode::OK)
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
    Json(req): Json<GetHostGroupRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    match state.db.get_host_group_by_name(&req.name).await? {
        None => {
            return Err(AntZookeeperError::validation_msg(&format!(
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
) -> Result<impl IntoResponse, AntZookeeperError> {
    if state.db.get_host_group_by_name(&req.name).await?.is_some() {
        return Err(AntZookeeperError::validation_msg(
            "Host group with that name already exists.",
        ));
    }

    let id = state.db.create_host_group(&req.name).await?;

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

async fn add_host_to_host_group(
    State(state): State<AntZookeeperState>,
    Json(req): Json<AddHostToHostGroupRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.host_group_exists_by_id(&req.host_group_id).await? {
        return Err(AntZookeeperError::validation_msg("No such host group."));
    }

    if !state.db.get_host(&req.host_id).await? {
        return Err(AntZookeeperError::validation_msg("No such host."));
    }

    if state
        .db
        .host_in_host_group(&req.host_group_id, &req.host_id)
        .await?
    {
        return Err(AntZookeeperError::validation_msg(
            "Host already in host group.",
        ));
    }

    info!("Adding host to group...");

    state
        .db
        .add_host_to_host_group(&req.host_group_id, &req.host_id)
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveHostFromHostGroupRequest {
    pub host_group_id: String,
    pub host_id: String,
}

async fn remove_host_from_host_group(
    State(state): State<AntZookeeperState>,
    Json(req): Json<RemoveHostFromHostGroupRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.host_group_exists_by_id(&req.host_group_id).await? {
        return Err(AntZookeeperError::validation_msg("No such host group."));
    }

    if !state.db.get_host(&req.host_id).await? {
        return Err(AntZookeeperError::validation_msg("No such host."));
    }

    state
        .db
        .remove_host_from_host_group(&req.host_group_id, &req.host_id)
        .await?;

    Ok(StatusCode::OK)
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
