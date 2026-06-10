use std::sync::Arc;

use ant_library::sd::reader::ServiceDiscovery;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::error;

pub type AntFsHostPorts = Vec<AntFsHostPort>;

#[derive(Serialize, Deserialize)]
pub struct AntFsHostPort {
    pub url: String,
    pub tls: bool,
}

#[derive(Clone)]
pub struct AntFsClient {
    sd: Option<Arc<ServiceDiscovery>>,

    host: Option<String>,
    port: Option<u16>,

    client: Client,
    username: String,
    password: String,
    use_tls: bool,
}

impl AntFsClient {
    pub fn new_via_sd(
        sd: Arc<ServiceDiscovery>,
        username: String,
        password: String,
        use_tls: bool,
    ) -> Self {
        Self {
            client: Client::new(),
            sd: Some(sd),
            host: None,
            port: None,
            username,
            password,
            use_tls,
        }
    }

    pub fn new(host: &str, port: u16, username: String, password: String, use_tls: bool) -> Self {
        Self {
            sd: None,
            client: Client::new(),
            use_tls,
            host: Some(host.to_string()),
            port: Some(port),
            username,
            password,
        }
    }

    fn tls(&self) -> &'static str {
        match self.use_tls {
            true => "s",
            false => "",
        }
    }

    async fn host_port(&self) -> Option<(String, u16)> {
        if let Some(sd) = &self.sd {
            let endpoint = sd.resolve(&"ant-fs").await;
            return endpoint.map(|e| (e.address, e.port));
        }

        let host = self.host.clone().unwrap(); // cannot happen based on constructors
        let port = self.port.unwrap();

        return Some((host, port));
    }

    async fn url(&self, path: &str) -> Result<String, anyhow::Error> {
        let address = self.host_port().await;

        match address {
            Some((host, port)) => Ok(format!("http{}://{}:{}/{}", self.tls(), host, port, path)),
            None => Err(anyhow::Error::msg(format!(
                "Unable to find endpoint for: {}",
                "ant-fs"
            ))),
        }
    }

    pub async fn delete_file(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let response = self
            .client
            .delete(self.url(path).await?)
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .send()
            .await?;

        match response.error_for_status() {
            Ok(_res) => Ok(()),
            Err(e) => {
                error!("Failed to put ant-fs file {e}");
                Err(e.into())
            }
        }
    }

    pub async fn put_file(&mut self, path: &str, bytes: Vec<u8>) -> Result<(), anyhow::Error> {
        let response = self
            .client
            .put(self.url(path).await?)
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .body(bytes)
            .send()
            .await?;

        match response.error_for_status() {
            Ok(_res) => Ok(()),
            Err(e) => {
                error!("Failed to put ant-fs file {e}");
                Err(e.into())
            }
        }
    }

    pub async fn get_file(&self, path: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let response = self
            .client
            .get(self.url(path).await?)
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => return Ok(Some(response.bytes().await?.to_vec())),
            StatusCode::NOT_FOUND => return Ok(None),
            _ => {
                let e = response.error_for_status().unwrap_err();
                error!("Failed to put ant-fs file {e}");
                return Err(e.into());
            }
        };
    }
}
