use ant_library::routes::Routes;
use axum::{
    extract::{Query, State},
    routing::get,
    Json,
};
use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{err::AntZookeeperError, state::AntZookeeperState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentView {
    pub build: Option<ProjectDeploymentRevision>,
    pub active_pipelines: Vec<ProjectDeploymentPipeline>,
    pub latest_finished_pipeline: Option<ProjectDeploymentPipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentRevision {
    pub revision_id: String,
    pub created_at: DateTime<Utc>,
    pub artifacts: Vec<ProjectDeploymentArtifact>,
    pub missing_architectures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentArtifact {
    pub architecture: String,
    pub build_version: String,
    pub size_bytes: i64,
    pub fingerprint: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentPipeline {
    pub pipeline_id: String,
    pub revision_id: String,
    pub state: String,
    pub layers: Vec<Vec<ProjectDeploymentNode>>,
    pub timeline: Vec<ProjectDeploymentTimelineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentTimelineEntry {
    pub node_id: String,
    pub to_state: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentNodeEvent {
    pub to_state: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum NodeState {
    Pending,
    Executable,
    InProgress,
    Finished,
    Failed { error: Option<String> },
    Cancelled,
    Unwound,
    UnwindFailed { error: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentNode {
    pub node_id: String,
    pub event: serde_json::Value,
    pub state: NodeState,
    pub resource_key: Option<String>,
    pub events: Vec<ProjectDeploymentNodeEvent>,
}

async fn get_project_deployments(
    State(state): State<AntZookeeperState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<(StatusCode, Json<ProjectDeploymentView>), AntZookeeperError> {
    let engine = &state.engine;

    let build = {
        let revision = state.db.get_latest_revision(&project_id).await?;
        match revision {
            Some((rev_id, None)) => {
                let artifacts = state.db.list_artifacts_for_revision_id(&rev_id).await?;
                let missing = state.db.missing_architectures_for_revision(&rev_id).await?;

                Some(ProjectDeploymentRevision {
                    revision_id: rev_id,
                    created_at: Utc::now(),
                    artifacts: artifacts
                        .into_iter()
                        .map(|(_, _, arch, version, size_bytes, fingerprint)| {
                            ProjectDeploymentArtifact {
                                architecture: arch.to_string(),
                                build_version: version,
                                size_bytes,
                                fingerprint,
                                registered_at: Utc::now(),
                            }
                        })
                        .collect(),
                    missing_architectures: missing.into_iter().map(|a| a.to_string()).collect(),
                })
            }
            _ => None,
        }
    };

    let active_pipelines = {
        let pipelines = engine.active_pipelines(&project_id).await?;
        let mut views = vec![];
        for p in pipelines {
            views.push(pipeline_to_view(engine, p).await?);
        }
        views
    };

    let latest_finished_pipeline = match engine.latest_finished_pipeline(&project_id).await? {
        None => None,
        Some(p) => Some(pipeline_to_view(engine, p).await?),
    };

    Ok((
        StatusCode::OK,
        Json(ProjectDeploymentView {
            build,
            active_pipelines,
            latest_finished_pipeline,
        }),
    ))
}

async fn pipeline_to_view(
    engine: &crate::pipeline_engine::engine::PipelineEngine,
    p: crate::pipeline_engine::engine::Pipeline,
) -> Result<ProjectDeploymentPipeline, anyhow::Error> {
    use std::collections::HashMap;

    let layers = engine.nodes_layered(&p.pipeline_id).await?;
    let events = engine.node_events(&p.pipeline_id).await?;
    let errors = engine.node_errors(&p.pipeline_id).await?;

    let timeline: Vec<ProjectDeploymentTimelineEntry> = events
        .iter()
        .map(|e| ProjectDeploymentTimelineEntry {
            node_id: e.node_id.clone(),
            to_state: e.to_state.clone(),
            reason: e.reason.clone(),
            created_at: e.created_at,
        })
        .collect();

    let mut events_by_node: HashMap<String, Vec<ProjectDeploymentNodeEvent>> = HashMap::new();
    for e in events {
        events_by_node
            .entry(e.node_id)
            .or_default()
            .push(ProjectDeploymentNodeEvent {
                to_state: e.to_state,
                reason: e.reason,
                created_at: e.created_at,
            });
    }

    Ok(ProjectDeploymentPipeline {
        pipeline_id: p.pipeline_id,
        revision_id: p.revision_id,
        state: p.state,
        timeline,
        layers: layers
            .into_iter()
            .map(|layer| {
                layer
                    .into_iter()
                    .map(|node| {
                        let node_events = events_by_node.remove(&node.node_id).unwrap_or_default();
                        let error = errors.get(&node.node_id).cloned();
                        node_to_view(node, error, node_events)
                    })
                    .collect()
            })
            .collect(),
    })
}

fn node_to_view(
    node: crate::pipeline_engine::engine::Node,
    error: Option<String>,
    events: Vec<ProjectDeploymentNodeEvent>,
) -> ProjectDeploymentNode {
    let event = serde_json::from_str(&node.event).unwrap_or(serde_json::Value::Null);
    let state = match node.state.as_str() {
        "pending" => NodeState::Pending,
        "executable" => NodeState::Executable,
        "in_progress" => NodeState::InProgress,
        "finished" => NodeState::Finished,
        "failed" => NodeState::Failed { error },
        "cancelled" => NodeState::Cancelled,
        "unwound" => NodeState::Unwound,
        "unwind_failed" => NodeState::UnwindFailed { error },
        unknown => panic!("unknown node state: {unknown}"),
    };
    ProjectDeploymentNode {
        node_id: node.node_id,
        event,
        state,
        resource_key: node.resource_key,
        events,
    }
}

pub fn routes() -> Routes<AntZookeeperState> {
    Routes::new().get("/{project_id}", get(get_project_deployments))
}
