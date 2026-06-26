use tracing::info;

use crate::{
    event_loop::{deploy::deploy_artifact, replicate::replicate_artifact_step},
    pipeline::deployment_event::DeploymentEvent,
    pipeline_engine::engine::{Dispatch, DispatchDirection},
    state::AntZookeeperState,
};

pub async fn dispatch(state: AntZookeeperState, d: Dispatch) -> Result<(), anyhow::Error> {
    let event: DeploymentEvent = serde_json::from_str(&d.node.event)?;

    match event {
        DeploymentEvent::ArtifactReplication {
            host_id,
            service_id,
            environment,
        } => match &d.direction {
            DispatchDirection::Deploy => {
                replicate_artifact_step(&state, &d.revision_id, &service_id, &environment, &host_id)
                    .await
            }
            DispatchDirection::Unwind { .. } => {
                info!(host_id = %host_id, service_id = %service_id, environment = %environment,
                        "ArtifactReplication unwind: no-op (artifacts are immutable)");
                Ok(())
            }
        },

        DeploymentEvent::HostDeployment {
            host_id,
            service_id,
            environment: _,
        } => {
            let target_revision = match &d.direction {
                DispatchDirection::Deploy => &d.revision_id,
                DispatchDirection::Unwind {
                    restore_revision_id,
                } => match restore_revision_id {
                    Some(rev) => rev,
                    None => {
                        info!(host_id = %host_id, service_id = %service_id,
                            "HostDeployment unwind: no previous revision, stopping service");
                        // TODO: implement service stop
                        return Ok(());
                    }
                },
            };

            let (_, arch) = state
                .db
                .get_host(&host_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("host not found: {host_id}"))?;

            let (_, version, _) = state
                .db
                .get_artifact_by_revision(target_revision, &service_id, Some(&arch))
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no artifact for revision={target_revision} service={service_id} arch={arch:?}"
                    )
                })?;

            deploy_artifact(&state, &service_id, &version, &host_id).await
        }

        DeploymentEvent::DeploymentVerification {
            host_id,
            service_id,
            environment,
        } => {
            info!(host_id = %host_id, service_id = %service_id, environment = %environment,
                "DeploymentVerification: not yet implemented");
            Ok(())
        }

        DeploymentEvent::RouteUpdate { environment } => {
            info!(environment = %environment, "RouteUpdate: not yet implemented");
            Ok(())
        }

        DeploymentEvent::AlertConfiguration {
            service_id,
            environment,
        } => {
            info!(service_id = %service_id, environment = %environment,
                "AlertConfiguration: not yet implemented");
            Ok(())
        }

        DeploymentEvent::LogRuleConfiguration {
            host_id,
            service_id,
        } => {
            info!(host_id = %host_id, service_id = %service_id,
                "LogRuleConfiguration: not yet implemented");
            Ok(())
        }

        DeploymentEvent::DatabaseMigration {
            service_id,
            environment,
        } => {
            info!(service_id = %service_id, environment = %environment,
                "DatabaseMigration: not yet implemented");
            Ok(())
        }

        DeploymentEvent::EnvironmentGate { from, to } => {
            info!(from = %from, to = %to, "EnvironmentGate: pass-through");
            Ok(())
        }
    }
}
