use ant_metadata::Project;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum KillStatus {
    Successful,
    NothingToKill,
    Unsuccessful,
}

#[derive(Serialize, Deserialize)]
pub struct KillProjectRequest {
    pub project: Project,
}

#[derive(Serialize, Deserialize)]
pub struct KillProjectResponse {
    pub status: KillStatus,
}
