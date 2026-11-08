use ant_archive_storage::ListBlobsResponse;
use reqwest::{Client, StatusCode};

mod tek;

#[derive(Debug, Clone)]
pub struct AntArchiveStorageNodeClient {
    pub node_id: String,
    pub host_id: String,
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

#[derive(thiserror::Error, Debug)]
pub enum AntArchiveStorageError {
    #[error("Error: request failed to ant-archive-storage: {0}")]
    Connection(#[from] reqwest::Error),

    #[error("Error({status}): {method} failed for key {storage_key}: {body}")]
    Failed {
        status: StatusCode,
        method: String,
        storage_key: String,
        body: String,
    },

    #[error("TEK encryption failed for key {0}")]
    Encryption(String),

    #[error("TEK decryption failed for key {0}")]
    Decryption(String),
}

impl AntArchiveStorageNodeClient {
    pub fn new(
        node_id: impl Into<String>,
        host_id: impl Into<String>,
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            host_id: host_id.into(),
            client: Client::new(),
            base_url: base_url.into(),
            username: username.into(),
            password: password.into(),
        }
    }

    pub async fn put(
        &self,
        storage_key: &str,
        tek: &[u8; 32],
        bytes: bytes::Bytes,
    ) -> Result<(), AntArchiveStorageError> {
        let tek_hex = base16ct::lower::encode_string(tek);
        let wire_payload = tek::wrap(tek, bytes.as_ref())?;

        let res = self
            .client
            .put(format!("{}/{}", self.base_url, storage_key))
            .basic_auth(&self.username, Some(&self.password))
            .header("X-Ant-Tek", tek_hex)
            .body(wire_payload)
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
            Err(AntArchiveStorageError::Failed {
                method: "PUT".to_string(),
                storage_key: storage_key.to_string(),
                status,
                body,
            })
        }
    }

    pub async fn get(
        &self,
        storage_key: &str,
        tek: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, AntArchiveStorageError> {
        let res = self
            .client
            .get(format!("{}/{}", self.base_url, storage_key))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;

        let status = res.status();
        let body = res.bytes().await?;

        match status {
            StatusCode::OK => Ok(Some(tek::unwrap(tek, storage_key, &body)?)),
            StatusCode::NOT_FOUND => Ok(None),
            s => Err(AntArchiveStorageError::Failed {
                method: "GET".to_string(),
                storage_key: storage_key.to_string(),
                status: s,
                body: String::from_utf8(body.to_vec())
                    .unwrap_or("<error failed to deserialize response>".to_string()),
            }),
        }
    }

    pub async fn delete(&self, storage_key: &str) -> Result<bool, AntArchiveStorageError> {
        let res = self
            .client
            .delete(format!("{}/{}", self.base_url, storage_key))
            .basic_auth(&self.username, Some(&self.password))
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
            s => Err(AntArchiveStorageError::Failed {
                method: "DELETE".to_string(),
                storage_key: storage_key.to_string(),
                status: s,
                body,
            }),
        }
    }

    /// One page of the blobs on this node, sorted by hash. Pass the response's `next` as
    /// `after` to get the following page.
    pub async fn list_blobs(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<ListBlobsResponse, AntArchiveStorageError> {
        let mut req = self
            .client
            .get(format!("{}/", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .query(&[("limit", limit.to_string())]);
        if let Some(after) = after {
            req = req.query(&[("after", after)]);
        }
        let res = req.send().await?;

        let status = res.status();
        if status == StatusCode::OK {
            return Ok(res.json().await?);
        }

        Err(AntArchiveStorageError::Failed {
            method: "LIST".to_string(),
            storage_key: after.unwrap_or_default().to_string(),
            status,
            body: res
                .text()
                .await
                .unwrap_or("<error failed to deserialize response>".to_string()),
        })
    }

    /// For blobs whose storage key isn't known, e.g. ones from `list_blobs` that no placement
    /// points to. Returns false if there was no blob with that hash.
    pub async fn delete_blob_by_hash(&self, hash: &str) -> Result<bool, AntArchiveStorageError> {
        let res = self
            .client
            .delete(format!("{}/blobs/{}", self.base_url, hash))
            .basic_auth(&self.username, Some(&self.password))
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
            s => Err(AntArchiveStorageError::Failed {
                method: "DELETE".to_string(),
                storage_key: hash.to_string(),
                status: s,
                body,
            }),
        }
    }
}
