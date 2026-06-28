use reqwest::{Client, StatusCode};

#[derive(Clone)]
pub struct AntArchiveStorageNodeClient {
    pub node_id: String,
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl AntArchiveStorageNodeClient {
    pub fn new(
        node_id: impl Into<String>,
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            client: Client::new(),
            base_url: base_url.into(),
            username: username.into(),
            password: password.into(),
        }
    }

    pub async fn put<'a>(
        &self,
        storage_key: &str,
        tek: &[u8],
        bytes: bytes::Bytes,
    ) -> Result<(), anyhow::Error> {
        let tek_hex = base16ct::lower::encode_string(tek);
        let res = self
            .client
            .put(format!("{}/{}", self.base_url, storage_key))
            .basic_auth(&self.username, Some(&self.password))
            .header("X-Ant-Tek", tek_hex)
            .body(bytes);

        let res = res.send().await?;

        if res.status() == StatusCode::CREATED {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "storage PUT failed for key {}: {}",
                storage_key,
                res.status()
            ))
        }
    }

    pub async fn get(&self, storage_key: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let res = self
            .client
            .get(format!("{}/{}", self.base_url, storage_key))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => Ok(Some(res.bytes().await?.to_vec())),
            StatusCode::NOT_FOUND => Ok(None),
            s => Err(anyhow::anyhow!(
                "storage GET failed for key {}: {}",
                storage_key,
                s
            )),
        }
    }

    pub async fn delete(&self, storage_key: &str) -> Result<bool, anyhow::Error> {
        let res = self
            .client
            .delete(format!("{}/{}", self.base_url, storage_key))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            s => Err(anyhow::anyhow!(
                "storage DELETE failed for key {}: {}",
                storage_key,
                s
            )),
        }
    }
}
