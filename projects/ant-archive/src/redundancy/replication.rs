use anyhow::Context;

use crate::{
    crypto::compute_checksum,
    redundancy::scheme::{RedundancyScheme, Shard, ShardKind},
};

pub struct Replication {
    n: i32,
}

impl Replication {
    pub fn new(n: i32) -> Self {
        Self { n }
    }
}

impl RedundancyScheme for Replication {
    fn id(&self) -> &'static str {
        "replication"
    }

    fn min_shards_to_reconstruct(&self) -> i32 {
        1
    }

    fn shard_count(&self) -> i32 {
        self.n
    }

    fn shard_kind(&self, _: i32) -> ShardKind {
        ShardKind::Data
    }

    fn shard(&self, data: &bytes::Bytes) -> Result<Vec<Shard>, anyhow::Error> {
        Ok((0..self.n)
            .into_iter()
            .map(|index| Shard {
                index,
                data: data.clone(),
                checksum: compute_checksum(data),
            })
            .collect())
    }

    fn unshard(&self, shards: Vec<Shard>) -> Result<bytes::Bytes, anyhow::Error> {
        shards
            .into_iter()
            .next()
            .map(|shard| shard.data)
            .context("no shards available")
    }

    fn regenerate_shard(&self, _: i32, available: &[Shard]) -> Result<bytes::Bytes, anyhow::Error> {
        available
            .into_iter()
            .next()
            .map(|shard| shard.data.clone())
            .context("no shards available")
    }
}
