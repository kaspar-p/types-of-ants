use std::collections::HashMap;

use reqwest::Client;

use rs_consul::{DeregisterEntityPayload, RegisterEntityPayload, RegisterEntityService};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug)]
pub struct ServiceDiscoveryWriter {
    consul_endpoint: String,
    client: Client,
}

/// From: https://developer.hashicorp.com/consul/api-docs/agent/service#json-request-body-schema
/// Can't use the one from rs_consul because it doesn't work, for some reason doesn't set the Address
/// correctly: https://github.com/Roblox/rs-consul
///
/// But keep this struct private.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RegisterServiceRequest {
    name: String,
    address: Option<String>,
    tags: Vec<String>,
    port: u16,
}

impl ServiceDiscoveryWriter {
    pub fn new(port: u16) -> Self {
        let consul_endpoint = format!("http://localhost:{port}");
        Self {
            consul_endpoint: consul_endpoint.clone(),
            client: Client::new(),
        }
    }

    pub async fn healthy(&self) -> bool {
        match reqwest::get(format!("{}/v1/agent/self", self.consul_endpoint))
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(_) => true,
            Err(e) => {
                error!("ANT-ERR-045: ant-matchmaker consul endpoint not healthy: {e}");
                false
            }
        }
    }

    async fn register_service(&self, req: RegisterServiceRequest) -> Result<(), anyhow::Error> {
        debug!("[consul register request] {}", serde_json::to_string(&req)?);
        let res = self
            .client
            .put(format!(
                "{}/v1/agent/service/register",
                self.consul_endpoint
            ))
            .json(&req)
            .send()
            .await?
            .error_for_status()?;

        let raw: String = res.text().await?;
        debug!("[consul register response] {}", raw);

        return Ok(());
    }

    pub async fn register_remote_service(
        &self,
        service: &str,
        node_name: &str,
        host: &str,
        port: u16,
    ) -> Result<(), anyhow::Error> {
        let req = RegisterEntityPayload {
            Node: node_name.to_string(),
            Address: host.to_string(),
            NodeMeta: HashMap::new(),
            ID: None,
            Service: Some(RegisterEntityService {
                Namespace: None,
                Service: service.to_string(),
                ID: None,
                TaggedAddresses: HashMap::new(),
                Tags: vec![],
                Meta: HashMap::new(),
                Port: Some(port),
            }),
            TaggedAddresses: HashMap::new(),
            Checks: vec![],
            SkipNodeUpdate: None,
            Datacenter: None,
        };
        debug!(
            "[consul catalog register request] {}",
            serde_json::to_string(&req)?
        );
        let res = self
            .client
            .put(format!("{}/v1/catalog/register", self.consul_endpoint))
            .json(&req)
            .send()
            .await?
            .error_for_status()?;

        let raw: String = res.text().await?;
        debug!("[consul catalog register response] {}", raw);

        return Ok(());
    }

    pub async fn deregister_remote_service(
        &self,
        service: &str,
        node_name: &str,
    ) -> Result<(), anyhow::Error> {
        let req = DeregisterEntityPayload {
            Namespace: None,
            Node: Some(node_name.to_string()),
            ServiceID: Some(service.to_string()),
            CheckID: None,
            Datacenter: None,
        };
        debug!(
            "[consul catalog deregister request] {}",
            serde_json::to_string(&req)?
        );
        let res = self
            .client
            .put(format!("{}/v1/catalog/deregister", self.consul_endpoint))
            .json(&req)
            .send()
            .await?
            .error_for_status()?;

        let raw: String = res.text().await?;
        debug!("[consul catalog deregister response] {}", raw);

        return Ok(());
    }

    pub async fn register_local_service(
        &self,
        service: &str,
        port: u16,
    ) -> Result<(), anyhow::Error> {
        let req = RegisterServiceRequest {
            address: None,
            name: service.to_string(),
            port,
            tags: vec!["typesofants:service".to_string()],
        };

        self.register_service(req).await
    }

    pub async fn deregister_local_service(&self, service: &str) -> Result<(), anyhow::Error> {
        debug!("[consul deregister request] {}", service);
        let res = self
            .client
            .put(format!(
                "{}/v1/agent/service/deregister/{}",
                self.consul_endpoint, service
            ))
            .send()
            .await?
            .error_for_status()?;

        let raw: String = res.text().await?;
        debug!("[consul deregister response] {}", raw);

        return Ok(());
    }
}
