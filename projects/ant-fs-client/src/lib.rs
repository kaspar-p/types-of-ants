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
    pub host: String,
    pub port: u16,

    client: Client,
    username: String,
    password: String,
    use_tls: bool,
}

impl AntFsClient {
    pub fn new(host: &str, port: u16, username: String, password: String, use_tls: bool) -> Self {
        Self {
            client: Client::new(),
            use_tls,
            host: host.to_string(),
            port,
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

    fn url(&self, path: &str) -> String {
        format!("http{}://{}:{}/{}", self.tls(), self.host, self.port, path)
    }

    pub async fn delete_file(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let response = self
            .client
            .delete(self.url(path))
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
            .put(self.url(path))
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
            .get(self.url(path))
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
