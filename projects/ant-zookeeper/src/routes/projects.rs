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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDeploymentNode {
    pub node_id: String,
    pub event: serde_json::Value,
    pub state: String,
    pub resource_key: Option<String>,
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
                let missing = state
                    .db
                    .missing_architectures_for_revision(&rev_id)
                    .await?;

                Some(ProjectDeploymentRevision {
                    revision_id: rev_id,
                    created_at: Utc::now(),
                    artifacts: artifacts
                        .into_iter()
                        .map(|(_, _, arch, version, size_bytes, fingerprint)| ProjectDeploymentArtifact {
                            architecture: arch.to_string(),
                            build_version: version,
                            size_bytes,
                            fingerprint,
                            registered_at: Utc::now(),
                        })
                        .collect(),
                    missing_architectures: missing
                        .into_iter()
                        .map(|a| a.to_string())
                        .collect(),
                })
            }
            _ => None,
        }
    };

    let active_pipelines = {
        let pipelines = engine.active_pipelines(&project_id).await?;
        let mut views = vec![];
        for p in pipelines {
            let layers = engine.nodes_layered(&p.pipeline_id).await?;
            views.push(ProjectDeploymentPipeline {
                pipeline_id: p.pipeline_id,
                revision_id: p.revision_id,
                state: p.state,
                layers: layers
                    .into_iter()
                    .map(|layer| layer.into_iter().map(node_to_view).collect())
                    .collect(),
            });
        }
        views
    };

    let latest_finished_pipeline = match engine.latest_finished_pipeline(&project_id).await? {
        None => None,
        Some(p) => {
            let layers = engine.nodes_layered(&p.pipeline_id).await?;
            Some(ProjectDeploymentPipeline {
                pipeline_id: p.pipeline_id,
                revision_id: p.revision_id,
                state: p.state,
                layers: layers
                    .into_iter()
                    .map(|layer| layer.into_iter().map(node_to_view).collect())
                    .collect(),
            })
        }
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

fn node_to_view(node: crate::pipeline_engine::engine::Node) -> ProjectDeploymentNode {
    let event = serde_json::from_str(&node.event).unwrap_or(serde_json::Value::Null);
    ProjectDeploymentNode {
        node_id: node.node_id,
        event,
        state: node.state,
        resource_key: node.resource_key,
    }
}

pub fn routes() -> Routes<AntZookeeperState> {
    Routes::new().get("/{project_id}", get(get_project_deployments))
}

