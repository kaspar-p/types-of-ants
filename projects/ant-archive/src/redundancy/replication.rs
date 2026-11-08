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

    fn shard_size(&self, chunk_size: usize) -> usize {
        chunk_size
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

#[cfg(test)]
mod test {
    use bytes::Bytes;

    use crate::redundancy::{replication::Replication, scheme::RedundancyScheme};

    #[test]
    fn shard_size_matches_shards() {
        let replication = Replication::new(3);

        for chunk_size in [0, 1, 26] {
            let shards = replication.shard(&Bytes::from(vec![7u8; chunk_size])).unwrap();

            for shard in shards {
                assert_eq!(replication.shard_size(chunk_size), shard.data.len());
            }
        }
    }
}
