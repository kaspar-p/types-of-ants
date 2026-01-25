use http::Method;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::routes::pipeline::{
    AddHostToHostGroupRequest, AddHostToHostGroupResponse, CreateHostGroupRequest,
    CreateHostGroupResponse, GetHostGroupRequest, GetHostGroupResponse, GetPipelineRequest,
    GetPipelineResponse, PutPipelineRequest, PutPipelineResponse, RemoveHostFromHostGroupRequest,
};

pub struct AntZookeeperClient {
    config: AntZookeeperClientConfig,
    client: reqwest::Client,
}

pub struct AntZookeeperClientConfig {
    pub tls: bool,
    pub endpoint: String,
}

pub struct AntZookeeperClientError<E> {
    error: E,
}

impl AntZookeeperClient {
    pub fn new(config: AntZookeeperClientConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn send<Req: Serialize, Res: for<'a> Deserialize<'a>>(
        &self,
        method: Method,
        path: &'static str,
        req: Req,
    ) -> Result<Res, anyhow::Error> {
        let endpoint = format!(
            "http{}://{}{}",
            match self.config.tls {
                true => "s",
                false => "",
            },
            self.config.endpoint,
            path
        );

        let res = self
            .client
            .request(method.clone(), &endpoint)
            .json(&req)
            .send()
            .await?
            .error_for_status();
        // .error_for_status()?
        // .json::<Res>()
        // .await;

        match res {
            Ok(res) => {
                let body = res.json::<Res>().await?;
                return Ok(body);
            }
            Err(err) => {
                error!("Error sending {} {}: {}", method.as_str(), endpoint, err);
                return Err(err.into());
            }
        }
    }

    pub async fn get_host_group(
        &self,
        req: GetHostGroupRequest,
    ) -> Result<GetHostGroupResponse, anyhow::Error> {
        self.send(Method::GET, "/pipeline/host-group/host-group", req)
            .await
    }

    pub async fn create_host_group(
        &self,
        req: CreateHostGroupRequest,
    ) -> Result<CreateHostGroupResponse, anyhow::Error> {
        self.send(Method::POST, "/pipeline/host-group/host-group", req)
            .await
    }

    pub async fn add_host_to_host_group(
        &self,
        req: AddHostToHostGroupRequest,
    ) -> Result<AddHostToHostGroupResponse, anyhow::Error> {
        self.send(Method::POST, "/pipeline/host-group/host", req)
            .await
    }

    pub async fn remove_host_from_host_group(
        &self,
        req: RemoveHostFromHostGroupRequest,
    ) -> Result<(), anyhow::Error> {
        self.send(Method::DELETE, "/pipeline/host-group/host", req)
            .await
    }

    pub async fn get_pipeline(
        &self,
        req: GetPipelineRequest,
    ) -> Result<GetPipelineResponse, anyhow::Error> {
        self.send(Method::GET, "/pipeline/pipeline", req).await
    }

    pub async fn put_pipeline(
        &self,
        req: PutPipelineRequest,
    ) -> Result<PutPipelineResponse, anyhow::Error> {
        self.send(Method::POST, "/pipeline/pipeline", req).await
    }
}
