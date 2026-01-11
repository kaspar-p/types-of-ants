use async_trait::async_trait;
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use tracing::info;

use crate::routes::service::InstallServiceRequest;

#[derive(Clone)]
pub struct AntHostAgentClient {
    pub client: Client,
    pub cfg: AntHostAgentClientConfig,
}

#[derive(Clone)]
pub struct AntHostAgentClientConfig {
    pub endpoint: String,
    pub port: u16,
}

impl AntHostAgentClient {
    fn endpoint(&self, path: &str) -> String {
        format!("http://{}:{}{path}", self.cfg.endpoint, self.cfg.port)
    }

    pub async fn register_service<R>(
        &self,
        project: &str,
        version: &str,
        mut service_file: R,
    ) -> Result<(), anyhow::Error>
    where
        R: std::io::Read,
    {
        let mut buf = Vec::new();
        service_file.read_to_end(&mut buf)?;
        let part = Part::bytes(buf);
        let form = Form::new().part("file", part);

        self.client
            .post(self.endpoint("/service/service-registration"))
            .header("X-Ant-Project", project)
            .header("X-Ant-Version", version)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn install_service(&self, req: InstallServiceRequest) -> Result<(), anyhow::Error> {
        info!(
            "ant_host_agent POST /service/service-installation : {}",
            serde_json::to_string(&req)?
        );
        self.client
            .post(self.endpoint("/service/service-installation"))
            .json(&req)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[async_trait]
pub trait AntHostAgentClientFactory: Send {
    async fn new_client(
        &self,
        cfg: AntHostAgentClientConfig,
    ) -> Result<AntHostAgentClient, anyhow::Error>;
}
pub struct RemoteAntHostAgentClientFactory;

#[async_trait]
impl AntHostAgentClientFactory for RemoteAntHostAgentClientFactory {
    async fn new_client(
        &self,
        cfg: AntHostAgentClientConfig,
    ) -> Result<AntHostAgentClient, anyhow::Error> {
        Ok(AntHostAgentClient {
            client: Client::new(),
            cfg,
        })
    }
}
