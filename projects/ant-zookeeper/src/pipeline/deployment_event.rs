use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeploymentEvent {
    ArtifactReplication {
        host_id: String,
        service_id: String,
    },
    HostDeployment {
        host_id: String,
        service_id: String,
    },
    DeploymentVerification {
        host_id: String,
        service_id: String,
    },
    RouteUpdate {
        environment: String,
    },
    AlertConfiguration {
        service_id: String,
        environment: String,
    },
    LogRuleConfiguration {
        host_id: String,
        service_id: String,
    },
    DatabaseMigration {
        service_id: String,
        environment: String,
    },
}
