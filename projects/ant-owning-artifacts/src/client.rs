use std::{ascii::EscapeDefault, time::Duration};

use anyhow::anyhow;
use axum::Json;
use reqwest::{Method, Url};
use tracing::{debug, warn};

use crate::routes::make::{MakeInput, MakeOutput};

pub struct AntOwningArtifactsClient {
    url: String,
    client: reqwest::Client,
}

pub enum AntOwningArtifactError {
    ConnectionError(reqwest::Error),
    MakeError(String),
}

impl Into<AntOwningArtifactError> for reqwest::Error {
    fn into(self) -> AntOwningArtifactError {
        AntOwningArtifactError::ConnectionError(self)
    }
}

impl AntOwningArtifactsClient {
    /// Create a new client for the ant-owning-artifacts webserver.
    /// host: Defaults to localhost
    /// port: Defaults to the value of the HOST_AGENT_PORT environment variable
    pub fn new(host: Option<String>, port: Option<u16>) -> Self {
        AntOwningArtifactsClient {
            url: format!(
                "http://{}:{}",
                host.unwrap_or("localhost".to_string()),
                port.unwrap_or(
                    dotenv::var("HOST_AGENT_PORT")
                        .expect("Could not find HOST_AGENT_PORT environment variable")
                        .parse::<u16>()
                        .expect("HOST_AGENT_PORT was not u16")
                )
            ),
            client: reqwest::Client::new(),
        }
    }

    pub async fn healthy(&self) -> bool {
        match self
            .client
            .get(format!("{}/ping", self.url))
            .timeout(Duration::from_millis(500))
            .send()
            .await
        {
            Err(e) => {
                warn!("ant-owning-artifacts failed to respond to ping: {}", e);
                return false;
            }
            Ok(v) => {
                let response_str = v.text().await.expect("/ping responds with text");
                debug!("ant-owning-artifacts is {}", response_str);
                return true;
            }
        }
    }

    pub async fn make(&self, input: MakeInput) -> Result<MakeOutput, anyhow::Error> {
        return Ok(self
            .client
            .post(format!("{}/api/make", self.url))
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(15 * 60))
            .body(serde_json::to_string(&input).expect("Input is not valid JSON"))
            .send()
            .await?
            .json::<MakeOutput>()
            .await?);
    }
}
