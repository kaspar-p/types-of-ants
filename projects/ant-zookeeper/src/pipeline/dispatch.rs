use tracing::info;

use crate::{
    event_loop::{deploy::deploy_artifact, replicate::replicate_artifact_step},
    pipeline::deployment_event::DeploymentEvent,
    pipeline_engine::engine::Node,
    state::AntZookeeperState,
};

pub struct DeploymentContext {
    pub revision_id: String,
}

pub async fn dispatch(
    state: AntZookeeperState,
    node: Node,
    context: DeploymentContext,
) -> Result<(), anyhow::Error> {
    let event: DeploymentEvent = serde_json::from_str(&node.event)?;

    match event {
        DeploymentEvent::ArtifactReplication { host_id, service_id, environment } => {
            replicate_artifact_step(&state, &context.revision_id, &service_id, &environment, &host_id).await
        }

        DeploymentEvent::HostDeployment { host_id, service_id, environment: _ } => {
            let (_, arch) = state
                .db
                .get_host(&host_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("host not found: {host_id}"))?;

            let (_, version, _) = state
                .db
                .get_artifact_by_revision(&context.revision_id, &service_id, Some(&arch))
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no artifact for revision={} service={service_id} arch={arch:?}",
                        context.revision_id
                    )
                })?;

            deploy_artifact(&state, &service_id, &version, &host_id).await
        }

        DeploymentEvent::DeploymentVerification { host_id, service_id, environment } => {
            info!(host_id = %host_id, service_id = %service_id, environment = %environment, "DeploymentVerification: not yet implemented");
            Ok(())
        }

        DeploymentEvent::RouteUpdate { environment } => {
            info!(environment = %environment, "RouteUpdate: not yet implemented");
            Ok(())
        }

        DeploymentEvent::AlertConfiguration { service_id, environment } => {
            info!(service_id = %service_id, environment = %environment, "AlertConfiguration: not yet implemented");
            Ok(())
        }

        DeploymentEvent::LogRuleConfiguration { host_id, service_id } => {
            info!(host_id = %host_id, service_id = %service_id, "LogRuleConfiguration: not yet implemented");
            Ok(())
        }

        DeploymentEvent::DatabaseMigration { service_id, environment } => {
            info!(service_id = %service_id, environment = %environment, "DatabaseMigration: not yet implemented");
            Ok(())
        }
    }
}
