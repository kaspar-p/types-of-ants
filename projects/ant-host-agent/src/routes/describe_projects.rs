use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct DescribeProjectsResponse {
    pub projects: Vec<String>,
}

pub async fn describe_projects() -> Json<DescribeProjectsResponse> {
    info!("Describing projects...");
    return Json(DescribeProjectsResponse {
        projects: vec!["ant-host-agent".to_string()],
    });
}
