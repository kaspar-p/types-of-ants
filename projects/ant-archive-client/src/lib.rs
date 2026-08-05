use std::sync::Arc;

use ant_library::sd::reader::ServiceDiscovery;
use reqwest::{Client, StatusCode};

#[derive(Clone)]
pub struct AntArchiveClient {
    client: Client,
    sd: Arc<ServiceDiscovery>,
    token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AntArchiveClientError {
    #[error("Error: request failed to ant-archive: {0}")]
    Connection(#[from] reqwest::Error),

    #[error("Error: no endpoint for ant-archive found in service discovery.")]
    AntArchiveNotFound,

    #[error("Error({status}): {method} failed for {bucket} {key}: {body}")]
    ObjectRequestFailed {
        status: StatusCode,
        method: String,
        bucket: String,
        key: String,
        body: String,
    },
}

impl AntArchiveClient {
    pub fn new(sd: Arc<ServiceDiscovery>, token: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            sd: sd,
            token: token.into(),
        }
    }

    async fn url(&self) -> Result<String, AntArchiveClientError> {
        let endpoint = self
            .sd
            .resolve("ant-archive")
            .await
            .ok_or(AntArchiveClientError::AntArchiveNotFound)?;

        Ok(format!("http://{}:{}", endpoint.address, endpoint.port))
    }

    pub async fn put_object<'a>(
        &self,
        bucket: &str,
        key: &str,
        bytes: bytes::Bytes,
    ) -> Result<(), AntArchiveClientError> {
        let res = self
            .client
            .put(format!("{}/o/{}/{}", self.url().await?, bucket, key))
            .bearer_auth(&self.token)
            .body(bytes)
            .send()
            .await?;

        let status = res.status();
        let body = res
            .text()
            .await
            .unwrap_or("<error failed to deserialize response>".to_string());

        if status == StatusCode::CREATED {
            Ok(())
        } else {
            Err(AntArchiveClientError::ObjectRequestFailed {
                method: "PUT".to_string(),
                bucket: bucket.to_string(),
                key: key.to_string(),
                status: status,
                body: body,
            })
        }
    }

    pub async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, AntArchiveClientError> {
        let res = self
            .client
            .get(format!("{}/o/{}/{}", self.url().await?, bucket, key))
            .bearer_auth(&self.token)
            .send()
            .await?;

        let status = res.status();
        let body = res.bytes().await?;

        match status {
            StatusCode::OK => Ok(Some(body.to_vec())),
            StatusCode::NOT_FOUND => Ok(None),
            s => Err(AntArchiveClientError::ObjectRequestFailed {
                method: "GET".to_string(),
                bucket: bucket.to_string(),
                key: key.to_string(),
                status: s,
                body: String::from_utf8(body.to_vec())
                    .unwrap_or("<error failed to deserialize response>".to_string()),
            }),
        }
    }

    pub async fn delete(&self, bucket: &str, key: &str) -> Result<bool, AntArchiveClientError> {
        let res = self
            .client
            .delete(format!("{}/o/{}/{}", self.url().await?, bucket, key))
            .bearer_auth(&self.token)
            .send()
            .await?;

        let status = res.status();
        let body = res
            .text()
            .await
            .unwrap_or("<error failed to deserialize response>".to_string());

        match status {
            StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            s => Err(AntArchiveClientError::ObjectRequestFailed {
                method: "DELETE".to_string(),
                bucket: bucket.to_string(),
                key: key.to_string(),
                status: s,
                body,
            }),
        }
    }
}
