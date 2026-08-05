use std::sync::Arc;

use ant_library::db::{
    database_connection, database_connection_dynamic, ConnectionPool, DatabaseConfig,
    DatabaseCredentialsConfig, TypesOfAntsDatabase,
};
use ant_library::sd::{pg::PoolError, reader::ServiceDiscovery};
use anyhow::Context;
use async_trait::async_trait;
use stdext::function_name;
use tracing::{debug, instrument};

#[derive(Debug, thiserror::Error)]
pub enum AntArchiveDbError {
    #[error("connection pool failed: {0}")]
    Connection(#[from] bb8::RunError<PoolError>),

    #[error("query failed: {0}")]
    Query(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct AntArchiveDb {
    pool: ConnectionPool,
}

#[derive(Debug)]
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
    // pub size_bytes: i64,
    pub kek_id: String,
    pub kek_alias: Option<String>,

    pub chunk_strategy: String,
    pub redundancy_strategy: String,

    /// Used like Decrypt(chunk_ciphertext, dek, nonce_prefix) => chunk_plaintext
    pub nonce_prefix: Vec<u8>,

    /// The DEK, encrypted like Encrypt(KEK, dek_nonce)
    pub encrypted_dek: Vec<u8>,
    pub dek_nonce: Vec<u8>,

    pub tek_derivation_key: Option<Vec<u8>>,
}

pub struct ArchivePlacement {
    pub storage_node_id: String,
    pub storage_key: String,
    pub checksum: String,
}

pub struct ShardPlacement {
    pub shard_id: String,
    pub shard_idx: i32,
    pub storage_node_id: String,
    pub storage_key: String,
    pub checksum: Vec<u8>,
}

pub struct ObjectChunk {
    pub chunk_id: String,
    pub chunk_idx: i32,
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

    #[instrument(skip(self))]
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
                "SELECT
                client_id, capability_can_select_storage_node
                FROM archive_client WHERE token_hash = $1",
                &[&hash],
            )
            .await
            .context(function_name!())?;

        Ok(row.map(|r| {
            (
                r.get("client_id"),
                ClientCapabilities {
                    can_select_storage_node: r.get("capability_can_select_storage_node"),
                },
            )
        }))
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?;
        Ok(())
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?;

        Ok(row.map(|r| ArchiveBucket {
            bucket_id: r.get("bucket_id"),
            client_id: r.get("client_id"),
            read_policy: r.get("read_policy"),
        }))
    }

    /// Returns (host_id, capacity_bytes)
    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?;

        Ok(row.map(|r| (r.get("host_id"), r.get("capacity_bytes"))))
    }

    pub async fn list_storage_nodes(&self) -> Result<Vec<String>, AntArchiveDbError> {
        let nodes = self
            .pool
            .get()
            .await?
            .query(
                "
                select storage_node_id
                from archive_storage_node
                ",
                &[],
            )
            .await
            .context(function_name!())?
            .into_iter()
            .map(|r| r.get("storage_node_id"))
            .collect();

        Ok(nodes)
    }

    /// Returns (node_id, protocol)
    /// where protocol is like 'http' or 'https' or something.
    #[instrument(skip(self))]
    pub async fn get_storage_node_by_node_name_or_id(
        &self,
        node_name_or_id: &str,
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
                    (host_id = $1 or storage_node_id = $1) and
                    is_active = true
                ",
                &[&node_name_or_id],
            )
            .await
            .context(function_name!())?;
        Ok(row.map(|r| (r.get("storage_node_id"), r.get("protocol"))))
    }

    /// Returns (kek_id, alias) where alias is the human-readable string
    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?;

        Ok(row.map(|r| (r.get("kek_id"), r.get("alias"))))
    }

    #[instrument(skip(self))]
    pub async fn get_current_object(
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
                select
                    obj.object_id,
                    obj.kek_id,
                    kek.alias,
                    obj.encrypted_dek,
                    obj.nonce_prefix,
                    obj.chunk_strategy,
                    obj.redundancy_strategy,
                    obj.dek_nonce,
                    obj.tek_derivation_key
                from archive_key key
                    join archive_object obj on key.current_object_id = obj.object_id
                    join archive_kek_version kek on obj.kek_id = kek.kek_id
                where
                    key.bucket_id = $1 and
                    key.key = $2 and
                    key.deleted_at is null
                ",
                &[&bucket_id, &key],
            )
            .await
            .context(function_name!())?;

        Ok(row.map(|r| ArchiveObject {
            object_id: r.get("object_id"),
            kek_id: r.get("kek_id"),
            kek_alias: r.get("alias"),
            chunk_strategy: r.get("chunk_strategy"),
            redundancy_strategy: r.get("redundancy_strategy"),
            nonce_prefix: r.get("nonce_prefix"),
            // size_bytes: r.get("size_bytes"),
            encrypted_dek: r.get("encrypted_dek"),
            dek_nonce: r.get("dek_nonce"),
            tek_derivation_key: r.get("tek_derivation_key"),
        }))
    }

    #[instrument(skip(self))]
    pub async fn list_chunks_for_object(
        &self,
        object_id: &str,
    ) -> Result<Vec<ObjectChunk>, AntArchiveDbError> {
        let chunks = self
            .pool
            .get()
            .await?
            .query(
                "
            select chunk_id, chunk_index
            from archive_chunk
            where
                object_id = $1
            ",
                &[&object_id],
            )
            .await
            .context(function_name!())?
            .into_iter()
            .map(|r| ObjectChunk {
                chunk_id: r.get("chunk_id"),
                chunk_idx: r.get("chunk_index"),
            })
            .collect();

        Ok(chunks)
    }

    /// Get the places where a given chunk is stored.
    #[instrument(skip(self))]
    pub async fn list_chunk_shard_placements(
        &self,
        chunk_id: &str,
    ) -> Result<Vec<ShardPlacement>, AntArchiveDbError> {
        let rows = self
            .pool
            .get()
            .await?
            .query(
                "
                select s.shard_id, p.storage_node_id, p.storage_key, s.shard_index, s.shard_checksum
                from archive_placement p
                    join archive_shard s on p.shard_id = s.shard_id
                where
                    s.chunk_id = $1
                order by s.shard_index asc
                ",
                &[&chunk_id],
            )
            .await
            .context(function_name!())?;

        Ok(rows
            .iter()
            .map(|r| ShardPlacement {
                shard_id: r.get("shard_id"),
                shard_idx: r.get("shard_index"),
                storage_node_id: r.get("storage_node_id"),
                storage_key: r.get("storage_key"),
                checksum: r.get("shard_checksum"),
            })
            .collect())
    }

    #[instrument(skip(self))]
    pub async fn bytes_stored_on_node(
        &self,
        storage_node_id: &str,
    ) -> Result<i64, AntArchiveDbError> {
        let bytes_stored = self
            .pool
            .get()
            .await
            .context(function_name!())?
            .query_one(
                "
                select
                    coalesce(sum(c.chunk_size_bytes), 0)::bigint as bytes_stored
                from archive_object o
                    join archive_key k on k.current_object_id = o.object_id
                    join archive_chunk c on c.object_id = o.object_id
                    join archive_shard s on c.chunk_id = s.chunk_id
                    join archive_placement p on s.shard_id = p.shard_id
                where
                    p.storage_node_id = $1 and
                    o.deleted_at is null
                ",
                &[&storage_node_id],
            )
            .await
            .context(function_name!())?
            .get::<_, i64>("bytes_stored");

        Ok(bytes_stored)
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?
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
            .await
            .context(function_name!())?
            .get("object_id");

        Ok(object_id)
    }

    /// During the lifecycle of an object, it starts PENDING before the
    /// bytes of the object are on all required nodes, then it can be closed.
    ///
    /// Also inserts a archive_key entry if there isn't already one.
    /// Must be closed with `complete_pending_object`.
    ///
    /// Returns (key_id, object_id)
    #[instrument(skip(self))]
    pub async fn insert_pending_object(
        &self,
        bucket_id: &str,
        kek_id: &str,
        key: &str,

        chunk_strategy: &str,
        redundancy_strategy: &str,

        encrypted_dek: &[u8],
        dek_nonce: &[u8],
        nonce_prefix: &[u8],
        tek_derivation_key: &[u8],
    ) -> Result<(String, String), AntArchiveDbError> {
        let mut con = self.pool.get().await?;
        let tx = con.transaction().await.context(function_name!())?;

        let key_id: Option<String> = tx
            .query_opt(
                "
                select key_id
                from archive_key
                where
                    bucket_id = $1 and
                    key = $2
                ",
                &[&bucket_id, &key],
            )
            .await
            .context(format!("{}: get-key", function_name!()))?
            .map(|r| r.get("key_id"));

        let key_id: String = match key_id {
            None => tx
                .query_one(
                    "
                    insert into archive_key
                        (bucket_id, key)
                    values
                        ($1, $2)
                    returning key_id
                    ",
                    &[&bucket_id, &key],
                )
                .await
                .context(format!("{}: write-key", function_name!()))?
                .get("key_id"),
            Some(key_id) => key_id,
        };

        let object_id = tx
            .query_one(
                "
                insert into archive_object (
                    bucket_id,
                    kek_id,
                    key_id,
                    key,
                    encrypted_dek,
                    dek_nonce,
                    tek_derivation_key,
                    chunk_strategy,
                    redundancy_strategy,
                    nonce_prefix
                )
                values
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                returning object_id
                ",
                &[
                    &bucket_id,
                    &kek_id,
                    &key_id,
                    &key,
                    &encrypted_dek,
                    &dek_nonce,
                    &tek_derivation_key,
                    &chunk_strategy,
                    &redundancy_strategy,
                    &nonce_prefix,
                ],
            )
            .await
            .context(format!("{}: write-obj", function_name!()))?
            .get("object_id");

        tx.commit().await.context(function_name!())?;

        Ok((key_id, object_id))
    }

    /// After `upsert_pending_object`, close the object. Also sets its version as "current version" for that key.
    #[instrument(skip(self))]
    pub async fn complete_pending_object(
        &self,
        object_id: &str,
        key_id: &str,
    ) -> Result<(), AntArchiveDbError> {
        let mut con = self.pool.get().await?;
        let tx = con.transaction().await.context(function_name!())?;

        tx.execute(
            "
        update archive_key
        set current_object_id = $1
        where key_id = $2
        ",
            &[&object_id, &key_id],
        )
        .await
        .context(function_name!())?;

        tx.commit().await.context(function_name!())?;

        Ok(())
    }

    /// During the lifecycle of a single chunk (of an object), it starts PENDING
    /// before being completed (after nodes have confirmed). Then can be closed.
    ///
    /// Returns chunk_id
    #[instrument(skip(self))]
    pub async fn upsert_pending_chunk(
        &self,
        object_id: &str,
        chunk_index: i32,
        chunk_size_bytes: i32,
    ) -> Result<String, AntArchiveDbError> {
        let mut con = self.pool.get().await?;
        let tx = con.transaction().await.context(function_name!())?;

        let chunk_id = tx
            .query_one(
                "
                insert into archive_chunk (
                    object_id,
                    chunk_index,
                    chunk_size_bytes
                )
                values
                    ($1, $2, $3)
                returning chunk_id
                ",
                &[&object_id, &chunk_index, &chunk_size_bytes],
            )
            .await
            .context(function_name!())?
            .get("chunk_id");

        tx.commit().await.context(function_name!())?;

        Ok(chunk_id)
    }

    /// After `upsert_pending_chunk`
    #[instrument(skip(self))]
    pub async fn complete_pending_chunk(&self, chunk_id: &str) -> Result<(), AntArchiveDbError> {
        let mut con = self.pool.get().await?;
        let tx = con.transaction().await.context(function_name!())?;

        tx.execute(
            "
        update archive_chunk
        set is_complete = true
        where chunk_id = $1
        ",
            &[&chunk_id],
        )
        .await
        .context(function_name!())?;

        tx.commit().await.context(function_name!())?;

        Ok(())
    }

    /// Create or update a shard's bytes on a single storage node
    #[instrument(skip(self))]
    pub async fn upsert_shard_placement(
        &self,
        chunk_id: &str,
        shard_idx: i32,
        storage_node_id: &str,
        storage_key: &str,
        shard_size: i64,
        checksum: &[u8],
    ) -> Result<(), AntArchiveDbError> {
        let mut con = self.pool.get().await?;
        let tx = con.transaction().await.context(function_name!())?;

        let shard_id: String = tx
            .query_one(
                "
            insert into archive_shard
                (chunk_id, shard_size_bytes, shard_index, shard_checksum)
            values
                ($1, $2, $3, $4)
            on conflict (chunk_id, shard_index)
            do update set
                shard_size_bytes = excluded.shard_size_bytes,
                shard_checksum = excluded.shard_checksum,
                updated_at = now()
            returning shard_id
            ",
                &[&chunk_id, &shard_size, &shard_idx, &checksum],
            )
            .await
            .context(function_name!())?
            .get("shard_id");

        tx.execute(
            "
            insert into archive_placement
                (shard_id, storage_node_id, storage_key)
            values
                ($1, $2, $3)
            on conflict (shard_id, storage_node_id)
            do update set
                storage_key = excluded.storage_key,
                updated_at = now()
            ",
            &[&shard_id, &storage_node_id, &storage_key],
        )
        .await
        .context(function_name!())?;

        tx.commit().await.context(function_name!())?;

        Ok(())
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?
            .get("kek_id");

        Ok(kek_id)
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?;
        Ok(())
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?
            .get("client_id");

        Ok(client_id)
    }

    #[instrument(skip(self))]
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
                "
                insert into archive_bucket
                    (bucket_id, client_id, is_default, read_policy)
                values
                    ($1, $2, $3, $4)
                ",
                &[&bucket_id, &client_id, &is_default, &read_policy],
            )
            .await
            .context(function_name!())?;
        Ok(())
    }

    #[instrument(skip(self))]
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
            .await
            .context(function_name!())?;

        Ok(rows
            .iter()
            .map(|r| ArchiveBucket {
                bucket_id: r.get("bucket_id"),
                client_id: r.get("client_id"),
                read_policy: r.get("read_policy"),
            })
            .collect())
    }

    /// Returns Some(String) if there was a previous object there.
    #[instrument(skip(self))]
    pub async fn soft_delete_key(
        &self,
        bucket_id: &str,
        key: &str,
    ) -> Result<Option<String>, AntArchiveDbError> {
        let previous_object_id = self
            .pool
            .get()
            .await?
            .query_opt(
                "
                with old as (
                    select current_object_id
                    from archive_key
                    where
                        bucket_id = $1 and
                        key = $2 and
                        deleted_at is null
                )
                update archive_key
                set
                    current_object_id = null,
                    deleted_at = now()
                where
                    bucket_id = $1 and
                    key = $2 and
                    deleted_at is null
                returning old.current_object_id as previous_object_id
                ",
                &[&bucket_id, &key],
            )
            .await
            .context(function_name!())?
            .map(|r| r.get("previous_object_id"));

        Ok(previous_object_id)
    }
}
