use std::sync::Arc;

use ant_library::db::{
    database_connection, database_connection_dynamic, ConnectionPool, DatabaseConfig,
    DatabaseCredentialsConfig, TypesOfAntsDatabase,
};
use ant_library::sd::{pg::PoolError, reader::ServiceDiscovery};
use async_trait::async_trait;
use tracing::debug;

#[derive(Debug, thiserror::Error)]
pub enum AntArchiveDbError {
    #[error("connection pool failed: {0}")]
    Connection(#[from] bb8::RunError<PoolError>),
    #[error("query failed: {0}")]
    Query(#[from] tokio_postgres::Error),
}

#[derive(Clone)]
pub struct AntArchiveDb {
    pool: ConnectionPool,
}

pub struct ClientCapabilities {
    pub can_select_storage_node: bool,
}

pub struct ArchiveBucket {
    pub bucket_id: String,
    pub client_id: String,
    pub read_policy: String,
}

pub struct ArchiveObject {
    pub object_id: String,
    pub kek_id: String,
    pub kek_alias: Option<String>,
    pub size_bytes: i64,
    pub encrypted_dek: Vec<u8>,
    pub dek_nonce: Vec<u8>,
    pub tek_derivation_key: Option<Vec<u8>>,
}

pub struct ArchivePlacement {
    pub storage_node_id: String,
    pub storage_key: String,
    pub object_checksum: String,
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

    pub async fn authenticate_bearer(
        &self,
        token: &str,
    ) -> Result<Option<(String, ClientCapabilities)>, AntArchiveDbError> {
        let hash = ant_library::crypto::make_token_hash(token);

        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT client_id, capability_can_select_storage_node FROM archive_client WHERE token_hash = $1",
                &[&hash],
            )
            .await?;

        Ok(row.map(|r| {
            (
                r.get("client_id"),
                ClientCapabilities {
                    can_select_storage_node: r.get("capability_can_select_storage_node"),
                },
            )
        }))
    }

    pub async fn set_client_capabilities(
        &self,
        client_id: &str,
        capabilities: &ClientCapabilities,
    ) -> Result<(), AntArchiveDbError> {
        self.pool
            .get()
            .await?
            .execute(
                "UPDATE archive_client SET capability_can_select_storage_node = $2 WHERE client_id = $1",
                &[&client_id, &capabilities.can_select_storage_node],
            )
            .await?;
        Ok(())
    }

    pub async fn get_bucket(
        &self,
        bucket_id: &str,
    ) -> Result<Option<ArchiveBucket>, AntArchiveDbError> {
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

    /// Returns (host_id, capacity_bytes)
    pub async fn describe_storage_node(
        &self,
        storage_node_id: &str,
    ) -> Result<Option<(String, i64)>, AntArchiveDbError> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                select host_id, capacity_bytes
                from archive_storage_node
                where
                    storage_node_id = $1
                ",
                &[&storage_node_id],
            )
            .await?;

        Ok(row.map(|r| (r.get("host_id"), r.get("capacity_bytes"))))
    }

    /// Returns (node_id, protocol)
    /// where protocol is like 'http' or 'https' or something.
    pub async fn get_storage_node_by_node_name(
        &self,
        node_name: &str,
    ) -> Result<Option<(String, String)>, AntArchiveDbError> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                select storage_node_id, protocol
                from archive_storage_node
                where
                    host_id = $1 and
                    is_active = true
                ",
                &[&node_name],
            )
            .await?;
        Ok(row.map(|r| (r.get("storage_node_id"), r.get("protocol"))))
    }

    /// Returns (kek_id, alias) where alias is the human-readable string
    pub async fn get_active_kek(
        &self,
    ) -> Result<Option<(String, Option<String>)>, AntArchiveDbError> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                select kek_id, alias
                from archive_kek_version
                where
                    is_active = true
                order by created_at desc
                limit 1",
                &[],
            )
            .await?;

        Ok(row.map(|r| (r.get("kek_id"), r.get("alias"))))
    }

    pub async fn get_object(
        &self,
        bucket_id: &str,
        key: &str,
    ) -> Result<Option<ArchiveObject>, AntArchiveDbError> {
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                select object_id, o.kek_id, k.alias, size_bytes, encrypted_dek, dek_nonce, tek_derivation_key
                from archive_object o
                    join archive_kek_version k on o.kek_id = k.kek_id
                where
                    o.bucket_id = $1 and
                    o.key = $2 and
                    o.deleted_at is null
                ",
                &[&bucket_id, &key],
            )
            .await?;

        Ok(row.map(|r| ArchiveObject {
            object_id: r.get("object_id"),
            kek_id: r.get("kek_id"),
            kek_alias: r.get("alias"),
            size_bytes: r.get("size_bytes"),
            encrypted_dek: r.get("encrypted_dek"),
            dek_nonce: r.get("dek_nonce"),
            tek_derivation_key: r.get("tek_derivation_key"),
        }))
    }

    pub async fn get_placements(
        &self,
        object_id: &str,
    ) -> Result<Vec<ArchivePlacement>, AntArchiveDbError> {
        let rows = self
            .pool
            .get()
            .await?
            .query(
                "SELECT storage_node_id, storage_key, object_checksum
                 FROM archive_object_placement
                 WHERE object_id = $1
                 ORDER BY idx ASC",
                &[&object_id],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|r| ArchivePlacement {
                storage_node_id: r.get("storage_node_id"),
                storage_key: r.get("storage_key"),
                object_checksum: r.get("object_checksum"),
            })
            .collect())
    }

    pub async fn bytes_stored_on_node(
        &self,
        storage_node_id: &str,
    ) -> Result<i64, AntArchiveDbError> {
        let bytes_stored = self
            .pool
            .get()
            .await?
            .query_one(
                "select coalesce(sum(o.size_bytes), 0)::bigint as bytes_stored
                 from archive_object o
                     join archive_object_placement p on o.object_id = p.object_id
                 where p.storage_node_id = $1
                   and o.deleted_at is null",
                &[&storage_node_id],
            )
            .await?
            .get::<_, i64>("bytes_stored");

        Ok(bytes_stored)
    }

    pub async fn upsert_object(
        &self,
        bucket_id: &str,
        kek_id: &str,
        key: &str,
        size_bytes: i64,
        encrypted_dek: &[u8],
        dek_nonce: &[u8],
        tek_derivation_key: &[u8],
    ) -> Result<String, AntArchiveDbError> {
        let object_id = self
            .pool
            .get()
            .await?
            .query_one(
                "
                insert into archive_object
                   (bucket_id, kek_id, key, size_bytes, encrypted_dek, dek_nonce, tek_derivation_key)
                values
                    ($1, $2, $3, $4, $5, $6, $7)
                on conflict (bucket_id, key)
                do update set
                    kek_id = EXCLUDED.kek_id,
                    size_bytes = EXCLUDED.size_bytes,
                    encrypted_dek = EXCLUDED.encrypted_dek,
                    dek_nonce = EXCLUDED.dek_nonce,
                    tek_derivation_key = EXCLUDED.tek_derivation_key,
                    updated_at = NOW(),
                    deleted_at = NULL
                returning object_id
                ",
                &[
                    &bucket_id,
                    &kek_id,
                    &key,
                    &size_bytes,
                    &encrypted_dek,
                    &dek_nonce,
                    &tek_derivation_key,
                ],
            )
            .await?
            .get("object_id");

        Ok(object_id)
    }

    pub async fn upsert_placement(
        &self,
        object_id: &str,
        storage_node_id: &str,
        storage_key: &str,
        checksum: &str,
        idx: i32,
    ) -> Result<(), AntArchiveDbError> {
        self.pool
            .get()
            .await?
            .execute(
                "insert into archive_object_placement
                   (object_id, storage_node_id, idx, role, storage_key, object_checksum)
                values ($1, $2, $3, 'replica', $4, $5)
                on conflict (object_id, idx) do update set
                    storage_node_id = EXCLUDED.storage_node_id,
                    storage_key = EXCLUDED.storage_key,
                    object_checksum = EXCLUDED.object_checksum",
                &[&object_id, &storage_node_id, &idx, &storage_key, &checksum],
            )
            .await?;

        Ok(())
    }

    pub async fn register_kek(&self, alias: &str) -> Result<String, AntArchiveDbError> {
        let kek_id = self
            .pool
            .get()
            .await?
            .query_one(
                "
                insert into archive_kek_version
                    (alias, is_active)
                values
                    ($1, true)
                returning kek_id
                ",
                &[&alias],
            )
            .await?
            .get("kek_id");

        Ok(kek_id)
    }

    pub async fn register_storage_node(
        &self,
        storage_node_id: &str,
        host_id: &str,
        capacity_bytes: i64,
        protocol: &str,
    ) -> Result<(), AntArchiveDbError> {
        self.pool
            .get()
            .await?
            .execute(
                "
                insert into archive_storage_node
                    (storage_node_id, host_id, capacity_bytes, protocol, is_active)
                values
                    ($1, $2, $3, $4, true)
                ",
                &[&storage_node_id, &host_id, &capacity_bytes, &protocol],
            )
            .await?;
        Ok(())
    }

    pub async fn create_client(
        &self,
        name: &str,
        token: &str,
    ) -> Result<String, AntArchiveDbError> {
        let token_hash = ant_library::crypto::make_token_hash(token);

        let client_id = self
            .pool
            .get()
            .await?
            .query_one(
                "
                insert into archive_client
                    (client_name, token_hash)
                values
                    ($1, $2)
                returning client_id
                ",
                &[&name, &token_hash],
            )
            .await?
            .get("client_id");

        Ok(client_id)
    }

    pub async fn create_bucket(
        &self,
        bucket_id: &str,
        client_id: &str,
        is_default: bool,
        read_policy: &str,
    ) -> Result<(), AntArchiveDbError> {
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

    pub async fn list_buckets_for_client(
        &self,
        client_id: &str,
    ) -> Result<Vec<ArchiveBucket>, AntArchiveDbError> {
        let rows = self
            .pool
            .get()
            .await?
            .query(
                "SELECT bucket_id, client_id, read_policy::text
                 FROM archive_bucket WHERE client_id = $1 ORDER BY bucket_id ASC",
                &[&client_id],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|r| ArchiveBucket {
                bucket_id: r.get("bucket_id"),
                client_id: r.get("client_id"),
                read_policy: r.get("read_policy"),
            })
            .collect())
    }

    pub async fn soft_delete_object(
        &self,
        bucket_id: &str,
        key: &str,
    ) -> Result<bool, AntArchiveDbError> {
        let count = self
            .pool
            .get()
            .await?
            .execute(
                "UPDATE archive_object SET deleted_at = NOW()
                 WHERE bucket_id = $1 AND key = $2 AND deleted_at IS NULL",
                &[&bucket_id, &key],
            )
            .await?;

        Ok(count > 0)
    }
}
