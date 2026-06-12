use std::sync::Arc;

use ant_library::db::{
    database_connection, database_connection_dynamic, ConnectionPool, DatabaseConfig,
    DatabaseCredentialsConfig, TypesOfAntsDatabase,
};
use ant_library::sd::reader::ServiceDiscovery;
use async_trait::async_trait;
use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha256};
use tracing::debug;

#[derive(Clone)]
pub struct AntArchiveDb {
    pool: ConnectionPool,
}

pub struct ArchiveBucket {
    pub bucket_id: String,
    pub client_id: String,
    pub read_policy: String,
}

pub struct ArchiveBlob {
    pub blob_id: String,
    pub kek_id: String,
    pub size_bytes: i64,
    pub encrypted_dek: Vec<u8>,
    pub dek_nonce: Vec<u8>,
}

pub struct ArchivePlacement {
    pub storage_node_id: String,
    pub storage_key: String,
}

#[async_trait]
impl TypesOfAntsDatabase for AntArchiveDb {
    async fn connect(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        debug!(
            "Connecting to database postgresql://{}:{}/{}",
            config.host, config.port, config.database_name
        );
        let pool = database_connection(config).await?;
        Ok(Self { pool })
    }
}

impl AntArchiveDb {
    pub async fn connect_discovered(sd: Arc<ServiceDiscovery>) -> Result<Self, anyhow::Error> {
        let pool = database_connection_dynamic(
            sd,
            "ant-archive-db",
            &DatabaseCredentialsConfig {
                database_name: ant_library::secret::load_secret("ant_archive_db_db")?,
                database_user: ant_library::secret::load_secret("ant_archive_db_user")?,
                database_password: ant_library::secret::load_secret("ant_archive_db_password")?,
                migration_dirs: vec![],
            },
        )
        .await?;
        Ok(Self { pool })
    }

    pub async fn authenticate_bearer(&self, token: &str) -> Result<Option<String>, anyhow::Error> {
        let hash = Sha256::digest(token.as_bytes());
        let hash_b64 = Base64::encode_string(&hash);

        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT client_id FROM archive_client WHERE token_hash = $1",
                &[&hash_b64],
            )
            .await?;

        Ok(row.map(|r| r.get("client_id")))
    }

    pub async fn get_bucket(
        &self,
        bucket_id: &str,
    ) -> Result<Option<ArchiveBucket>, anyhow::Error> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT bucket_id, client_id, read_policy::text
                 FROM archive_bucket WHERE bucket_id = $1",
                &[&bucket_id],
            )
            .await?;

        Ok(row.map(|r| ArchiveBucket {
            bucket_id: r.get("bucket_id"),
            client_id: r.get("client_id"),
            read_policy: r.get("read_policy"),
        }))
    }

    pub async fn get_storage_node_by_node_name(
        &self,
        node_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT storage_node_id FROM archive_storage_node
                 WHERE host_id = $1 AND is_active = true",
                &[&node_name],
            )
            .await?;
        Ok(row.map(|r| r.get("storage_node_id")))
    }

    pub async fn get_active_kek_id(&self) -> Result<Option<String>, anyhow::Error> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT kek_id FROM archive_kek_version WHERE is_active = true LIMIT 1",
                &[],
            )
            .await?;

        Ok(row.map(|r| r.get("kek_id")))
    }

    pub async fn get_blob(
        &self,
        bucket_id: &str,
        key: &str,
    ) -> Result<Option<ArchiveBlob>, anyhow::Error> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT blob_id, kek_id, size_bytes, encrypted_dek, dek_nonce
                 FROM archive_blob
                 WHERE bucket_id = $1 AND key = $2 AND deleted_at IS NULL",
                &[&bucket_id, &key],
            )
            .await?;

        Ok(row.map(|r| ArchiveBlob {
            blob_id: r.get("blob_id"),
            kek_id: r.get("kek_id"),
            size_bytes: r.get("size_bytes"),
            encrypted_dek: r.get("encrypted_dek"),
            dek_nonce: r.get("dek_nonce"),
        }))
    }

    pub async fn get_placements(
        &self,
        blob_id: &str,
    ) -> Result<Vec<ArchivePlacement>, anyhow::Error> {
        let rows = self
            .pool
            .get()
            .await?
            .query(
                "SELECT storage_node_id, storage_key
                 FROM archive_blob_placement
                 WHERE blob_id = $1",
                &[&blob_id],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|r| ArchivePlacement {
                storage_node_id: r.get("storage_node_id"),
                storage_key: r.get("storage_key"),
            })
            .collect())
    }

    pub async fn upsert_blob(
        &self,
        bucket_id: &str,
        kek_id: &str,
        key: &str,
        size_bytes: i64,
        encrypted_dek: &[u8],
        dek_nonce: &[u8],
    ) -> Result<String, anyhow::Error> {
        let candidate_id = generate_id("blob");

        let row = self
            .pool
            .get()
            .await?
            .query_one(
                "INSERT INTO archive_blob
                   (blob_id, bucket_id, kek_id, key, size_bytes, encrypted_dek, dek_nonce)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (bucket_id, key) DO UPDATE SET
                   kek_id = EXCLUDED.kek_id,
                   size_bytes = EXCLUDED.size_bytes,
                   encrypted_dek = EXCLUDED.encrypted_dek,
                   dek_nonce = EXCLUDED.dek_nonce,
                   updated_at = NOW(),
                   deleted_at = NULL
                 RETURNING blob_id",
                &[
                    &candidate_id,
                    &bucket_id,
                    &kek_id,
                    &key,
                    &size_bytes,
                    &encrypted_dek,
                    &dek_nonce,
                ],
            )
            .await?;

        Ok(row.get("blob_id"))
    }

    pub async fn upsert_placement(
        &self,
        blob_id: &str,
        storage_node_id: &str,
        storage_key: &str,
        checksum: &str,
    ) -> Result<(), anyhow::Error> {
        let placement_id = generate_id("pl");

        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO archive_blob_placement
                   (placement_id, blob_id, storage_node_id, idx, role, storage_key, checksum)
                 VALUES ($1, $2, $3, 0, 'replica', $4, $5)
                 ON CONFLICT (blob_id, idx) DO UPDATE SET
                   storage_key = EXCLUDED.storage_key,
                   checksum = EXCLUDED.checksum",
                &[
                    &placement_id,
                    &blob_id,
                    &storage_node_id,
                    &storage_key,
                    &checksum,
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn register_kek(&self, kek_id: &str) -> Result<(), anyhow::Error> {
        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO archive_kek_version (kek_id, is_active) VALUES ($1, true)",
                &[&kek_id],
            )
            .await?;
        Ok(())
    }

    pub async fn register_storage_node(
        &self,
        storage_node_id: &str,
        host_id: &str,
    ) -> Result<(), anyhow::Error> {
        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO archive_storage_node (storage_node_id, host_id, is_active)
                 VALUES ($1, $2, true)",
                &[&storage_node_id, &host_id],
            )
            .await?;
        Ok(())
    }

    pub async fn create_client(
        &self,
        client_id: &str,
        name: &str,
        token_hash: &str,
    ) -> Result<(), anyhow::Error> {
        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO archive_client (client_id, client_name, token_hash)
                 VALUES ($1, $2, $3)",
                &[&client_id, &name, &token_hash],
            )
            .await?;
        Ok(())
    }

    pub async fn create_bucket(
        &self,
        bucket_id: &str,
        client_id: &str,
        is_default: bool,
        read_policy: &str,
    ) -> Result<(), anyhow::Error> {
        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO archive_bucket (bucket_id, client_id, is_default, read_policy)
                 VALUES ($1, $2, $3, $4)",
                &[&bucket_id, &client_id, &is_default, &read_policy],
            )
            .await?;
        Ok(())
    }

    pub async fn soft_delete_blob(
        &self,
        bucket_id: &str,
        key: &str,
    ) -> Result<bool, anyhow::Error> {
        let count = self
            .pool
            .get()
            .await?
            .execute(
                "UPDATE archive_blob SET deleted_at = NOW()
                 WHERE bucket_id = $1 AND key = $2 AND deleted_at IS NULL",
                &[&bucket_id, &key],
            )
            .await?;

        Ok(count > 0)
    }
}

pub fn generate_id(prefix: &str) -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{}-{}", prefix, base16ct::lower::encode_string(&bytes))
}
