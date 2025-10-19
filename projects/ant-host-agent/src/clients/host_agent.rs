use crate::routes::service::{DisableServiceRequest, EnableServiceRequest};

use super::host::Host;

use anyhow::Result;
use hyper::Method;
use std::time::Duration;
use tracing::debug;

pub struct HostAgentClient {
    pub host: Host,
    client: reqwest::Client,
}

impl HostAgentClient {
    pub fn connect(host: Host) -> Result<HostAgentClient> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;

        Ok(HostAgentClient { host, client })
    }

    pub async fn healthy(&self) -> bool {
        return self.ping().await.is_ok();
    }

    pub async fn ping(&self) -> Result<()> {
        let req = self
            .client
            .request(
                Method::GET,
                self.host.http_endpoint(Some("ping".to_owned())),
            )
            .build()?;
        let res = self.client.execute(req).await?;
        let data = res.text().await?;
        debug!("Ping string: '{}'", data);

        match data.as_str() {
            "healthy ant" => return Ok(()),
            other => return Err(anyhow::Error::msg(format!("No healthy string: {}", other))),
        };
    }

    pub async fn enable_service(&self, req: EnableServiceRequest) -> Result<()> {
        let req = self
            .client
            .post(self.host.http_endpoint(Some("service/service".to_owned())))
            .json(&req)
            .build()?;

        let res = self.client.execute(req).await?;
        assert!(res.status().is_success());

        return Ok(());
    }

    pub async fn disable_service(&self, req: DisableServiceRequest) -> Result<()> {
        let req = self
            .client
            .delete(self.host.http_endpoint(Some("service/service".to_owned())))
            .json(&req)
            .build()?;

        let res = self.client.execute(req).await?;
        assert!(res.status().is_success());

        return Ok(());
    }
}
