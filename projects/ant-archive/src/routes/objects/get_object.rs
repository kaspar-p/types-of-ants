use std::collections::VecDeque;

use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
use bytes::Bytes;
use futures::stream::{self};
use http::StatusCode;
use sha2::{Digest, Sha256};
use tracing::error;

use crate::{
    auth::BearerClaims,
    chunker::{self, chunker::Chunker},
    err::AntArchiveError,
    placement::resolve_storage_nodes,
    redundancy::{
        self,
        scheme::{RedundancyScheme, Shard, ShardKind},
    },
    routes::objects::{kek, tek},
    state::AntArchiveState,
};

fn compute_checksum(body: &[u8]) -> Vec<u8> {
    Sha256::digest(body).to_vec()
}

// One immutable snapshot of everything needed to stream an object's plaintext,
// resolved once up front so the per-chunk loop below never touches the DB
// for anything except the chunk/shard rows themselves.
struct ReadContext {
    state: AntArchiveState,
    dek: [u8; 32],
    nonce_prefix: [u8; 4],
    chunker: Box<dyn Chunker>,
    redundancy: Box<dyn RedundancyScheme>,
    remaining: VecDeque<ChunkMeta>,
}

struct ChunkMeta {
    chunk_id: String,
    chunk_idx: i32,
    is_last: bool,
}

pub(super) async fn get_object(
    State(state): State<AntArchiveState>,
    Path((bucket_id, key)): Path<(String, String)>,
    maybe_auth: Option<BearerClaims>,
) -> Result<Response, AntArchiveError> {
    let bucket = state
        .db
        .get_bucket(&bucket_id)
        .await?
        .ok_or_else(|| AntArchiveError::BucketNotFound(bucket_id.clone()))?;

    match bucket.read_policy.as_str() {
        "public" => {}
        "internal" => {
            if maybe_auth.is_none() {
                return Err(AntArchiveError::BucketNotFound(bucket_id.clone()));
            }
        }
        "private" => {
            let not_found = || AntArchiveError::ObjectNotFound(key.clone());
            let auth = maybe_auth.ok_or_else(&not_found)?;
            if bucket.client_id != auth.client_id {
                return Err(not_found());
            }
        }
        _ => {
            return Err(AntArchiveError::InternalServerError(
                "ANT-ERR-107",
                Some(anyhow::anyhow!("unknown read policy")),
            ))
        }
    }

    // Resolve via the archive_key pointer, NOT a direct (bucket_id, key) -> object lookup
    let object = state
        .db
        .get_current_object(&bucket_id, &key)
        .await?
        .ok_or_else(|| AntArchiveError::ObjectNotFound(key.clone()))?;

    let chunks = state.db.list_chunks_for_object(&object.object_id).await?;
    if chunks.is_empty() {
        return Err(AntArchiveError::InternalServerError(
            "ANT-ERR-108",
            Some(anyhow::anyhow!("object has no chunks")),
        ));
    }
    let last_index = chunks.iter().map(|c| c.chunk_idx).max().unwrap();

    let tek = tek::derive_tek(&object.tek_derivation_key.expect("objects should have teks"))?;
    let kek = kek::load_kek(&object.kek_id, object.kek_alias.as_deref())?;
    let dek = kek::decrypt_dek(
        &kek,
        &kek::EncryptedDek {
            dek_nonce: object.dek_nonce,
            dek_ciphertext: object.encrypted_dek,
        },
    )?;

    let chunker: Box<dyn Chunker> = chunker::from_id(&state, &object.chunk_strategy)?;
    let redundancy: Box<dyn RedundancyScheme> = redundancy::from_id(&object.redundancy_strategy)?;

    let ctx = ReadContext {
        state,
        dek,
        nonce_prefix: object.nonce_prefix.as_slice()[..4]
            .try_into()
            .expect("ANT-ERR-137: nonce_prefix was not 4 bytes long"),
        chunker,
        redundancy,
        remaining: chunks
            .into_iter()
            .map(|c| ChunkMeta {
                chunk_id: c.chunk_id,
                chunk_idx: c.chunk_idx,
                is_last: c.chunk_idx == last_index,
            })
            .collect(),
    };

    let storage_nodes = resolve_storage_nodes(&ctx.state).await?;

    // Lazy, per-chunk stream: nothing beyond this point buffers the whole object
    let body_stream = stream::try_unfold(ctx, move |mut ctx| {
        let value = storage_nodes.clone();
        async move {
            let Some(meta) = ctx.remaining.pop_front() else {
                return Ok(None); // no chunks left -> end of stream
            };

            let placements = ctx
                .state
                .db
                .list_chunk_shard_placements(&meta.chunk_id)
                .await?;

            // Prefer directly-usable shards (replicas, or ECC data shards) before
            // reaching for anything that requires reconstruction math.
            let mut ordered = placements;
            ordered.sort_by_key(|p| ctx.redundancy.shard_kind(p.shard_idx) != ShardKind::Data);

            let mut good: Vec<Shard> = Vec::new();
            for p in ordered {
                if good.len() >= ctx.redundancy.min_shards_to_reconstruct() as usize {
                    break;
                }

                let Some(node) = value.get(&p.storage_node_id) else {
                    continue;
                };

                let Some(bytes) = node.client.get(&p.storage_key, &tek).await? else {
                    error!(
                        node_id = %p.storage_node_id, storage_key = %p.storage_key,
                        "ANT-ERR-002: blob missing from storage node: \
                        placement record exists but data does not"
                    );
                    continue;
                };
                let checksum = compute_checksum(&bytes);
                if checksum != p.checksum {
                    error!(
                        node_id = %p.storage_node_id, storage_key = %p.storage_key,
                        expected = %base16ct::lower::encode_string(&p.checksum), actual = %base16ct::lower::encode_string(&checksum),
                        "ANT-ERR-003: shard checksum mismatch"
                    );
                    continue;
                }

                good.push(Shard {
                    index: p.shard_idx,
                    data: Bytes::from(bytes),
                    checksum,
                });
            }

            if good.len() < ctx.redundancy.min_shards_to_reconstruct() as usize {
                return Err(AntArchiveError::InternalServerError(
                    "ANT-ERR-005",
                    Some(anyhow::anyhow!(
                        "chunk {} unreadable: got {} usable shards, needed {}",
                        meta.chunk_id,
                        good.len(),
                        ctx.redundancy.min_shards_to_reconstruct()
                    )),
                ));
            }

            let ciphertext = ctx
                .redundancy
                .unshard(good)
                .context("reconstructing chunk")?;
            let plaintext = ctx
                .chunker
                .decrypt_chunk(
                    &ctx.dek,
                    &ctx.nonce_prefix,
                    meta.chunk_idx as u64,
                    meta.is_last,
                    ciphertext,
                )
                .context("decrypting chunk")?;

            Ok(Some((plaintext, ctx)))
        }
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from_stream(body_stream))
        .context("failed to build response")?)
}
