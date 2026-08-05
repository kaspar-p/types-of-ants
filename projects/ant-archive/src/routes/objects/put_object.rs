use std::collections::HashSet;

use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::{headers::ContentLength, TypedHeader};
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use http::StatusCode;
use tracing::{info, warn};

use crate::{
    auth::BearerClaims,
    chunker::{self, chunker::Chunker},
    crypto,
    err::AntArchiveError,
    headers::SelectStorageNode,
    placement::{self, Placement},
    redundancy::{self, scheme::RedundancyScheme},
    routes::objects::{kek, tek},
    state::AntArchiveState,
};

fn body_to_io_stream(body: Body) -> BoxStream<'static, std::io::Result<Bytes>> {
    Box::pin(
        body.into_data_stream()
            .map(|res| res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))),
    )
}

fn validate_key(key: &str) -> Result<(), AntArchiveError> {
    if key.is_empty() {
        return Err(AntArchiveError::BadRequest(
            "key must not be empty".to_string(),
        ));
    }
    if key.starts_with('/') {
        return Err(AntArchiveError::BadRequest(
            "key must not start with '/'".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn put_object(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    content_length: Option<TypedHeader<ContentLength>>,
    auth: BearerClaims,
    select_node: Option<SelectStorageNode>,
    body: Body,
) -> Result<impl IntoResponse, AntArchiveError> {
    // VALIDATION
    {
        validate_key(&key)?;

        let bucket = state
            .db
            .get_bucket(&bucket_id)
            .await?
            .ok_or_else(|| AntArchiveError::BucketNotFound(bucket_id.clone()))?;

        if bucket.client_id != auth.client_id {
            return Err(AntArchiveError::BucketNotFound(bucket_id.clone()));
        }

        if let Some(selected) = &select_node {
            state
                .db
                .get_storage_node_by_node_name_or_id(&selected.0)
                .await?
                .ok_or(AntArchiveError::BadRequest(format!(
                    "No such storage node: {}",
                    selected.0
                )))?;
        }
    }

    let (kek_id, kek_alias) = state.db.get_active_kek().await?.ok_or_else(|| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-105",
            Some(anyhow::anyhow!("no active KEK version")),
        )
    })?;

    // CHOOSE PARAMETERS
    let content_length = content_length.map(|h| h.0 .0).unwrap_or(0);

    let chunker: Box<dyn Chunker> = Box::new(chunker::fixed_size::FixedSize::new(state.chunk_size));
    let redundancy: Box<dyn RedundancyScheme> =
        Box::new(redundancy::replication::Replication::new(3));

    let dek = crypto::generate_random_32(state.rng.as_ref());
    let tek_derivation_key = crypto::generate_random_32(&*state.rng);
    let tek = tek::derive_tek(&tek_derivation_key)?;

    let nonce_prefix = crypto::generate_random_4(state.rng.as_ref());

    let encrypted_dek = kek::encrypt_dek(&kek::load_kek(&kek_id, kek_alias.as_deref())?, &dek)?;

    let (key_id, object_id) = state
        .db
        .insert_pending_object(
            &bucket_id,
            &kek_id,
            &key,
            chunker.id(),
            redundancy.id(),
            &encrypted_dek.dek_ciphertext,
            &encrypted_dek.dek_nonce,
            &nonce_prefix,
            &tek_derivation_key,
        )
        .await?;

    let mut placements: Vec<Placement> = placement::place_group(
        &state,
        &object_id,
        content_length as usize,
        redundancy.shard_count(),
        select_node.map(|n| n.0).as_deref(),
    )
    .await?;
    let mut disqualified: HashSet<String> = placements
        .iter()
        .map(|n| n.node.node_id.to_string())
        .collect();

    // STREAM + ENCRYPT CHUNKS TO STORAGE

    // L1: Each object broken into chunks (for constant-size memory purposes)
    let mut chunks = chunker.encrypt_stream(dek, nonce_prefix, body_to_io_stream(body));
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.context("encryption failed")?;

        let chunk_id = state
            .db
            .upsert_pending_chunk(&object_id, chunk.index as i32, chunk.plaintext_len as i32)
            .await?;

        let shards = redundancy.shard(&chunk.ciphertext).context("shard chunk")?;

        assert_eq!(
            shards.len(),
            placements.len(),
            "ANT-ERR-135: Broke chunk {chunk_id} into {} shards but trying to place on {} nodes",
            shards.len(),
            placements.len()
        );

        // L2: Each chunk broken into shards (redundancy to distinct nodes)
        for (i, shard) in shards.into_iter().enumerate() {
            let shard_size = shard.data.len();
            let storage_key = format!("{object_id}-{:08}-{}", chunk.index, shard.index);

            // Loop until we successfully place onto the node, without needing a replacement
            loop {
                let placement = &placements[i];
                info!(
                    "[inpr] Placing {chunk_id}[{}] shard {} (l={}) onto {} ({})",
                    chunk.index,
                    shard.index,
                    shard_size,
                    placement.node.host_id,
                    placement.node.node_id
                );

                let res = placement
                    .node
                    .put(&storage_key, &tek, shard.data.clone())
                    .await
                    .with_context(|| {
                        format!(
                            "{} PUT to {} ({}) for shard {} (l={})",
                            chunk_id,
                            placement.node.node_id,
                            placement.node.host_id,
                            shard.index,
                            shard_size
                        )
                    });

                match res {
                    Ok(()) => {
                        state
                            .db
                            .upsert_shard_placement(
                                &chunk_id,
                                shard.index,
                                &placement.node.node_id,
                                &storage_key,
                                shard_size as i64,
                                &shard.checksum,
                            )
                            .await?;
                        info!(
                            "[done] Placing {chunk_id}[{}] shard {} (l={}) onto {} ({})",
                            chunk.index,
                            shard.index,
                            shard_size,
                            placement.node.host_id,
                            placement.node.node_id
                        );
                        break;
                    }
                    Err(e) => {
                        warn!("Failed to PUT to node, finding replacement for [i={i}]: {e:?}",);
                        disqualified.insert(placement.node.node_id.to_string());
                        placements[i] = placement::find_replacement(
                            &state,
                            &object_id,
                            content_length as usize,
                            &disqualified,
                        )
                        .await?;

                        // Loop again to place at new object
                    }
                }
            }
        }

        info!("Marking chunk {chunk_id} [{}] complete.", chunk.index);
        state
            .db
            .complete_pending_chunk(&chunk_id)
            .await
            .with_context(|| format!("complete chunk {} {chunk_id}", chunk.index))?;
    }

    info!("Marking object {object_id} complete and transitioning its key version");
    state
        .db
        .complete_pending_object(&object_id, &key_id)
        .await
        .with_context(|| format!("complete object {bucket_id} {key} {object_id}"))?;

    Ok(StatusCode::CREATED)
}
