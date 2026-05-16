use ant_host_agent::client::AntHostAgentClientConfig;

use crate::state::AntZookeeperState;

pub async fn deploy_artifact(
    state: &AntZookeeperState,
    project: &str,
    version: &str,
    host: &str,
) -> Result<(), anyhow::Error> {
    let ant_host_agent =
        state
            .ant_host_agent_factory
            .lock()
            .await
            .new_client(AntHostAgentClientConfig {
                endpoint: host.to_string(),
                port: 3232,
            });

    ant_host_agent
        .enable_service(ant_host_agent::routes::service::EnableServiceRequest {
            service_id: Some(project.to_string()),
            project: Some(project.to_string()),
            version: version.to_string(),
        })
        .await?;

    Ok(())
}
