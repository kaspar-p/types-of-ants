use bytes::Bytes;

pub struct Shard {
    pub index: i32,
    pub data: Bytes,
    pub checksum: Vec<u8>,
}

#[derive(PartialEq, Eq)]
pub enum ShardKind {
    Data,
    Redundancy,
}

pub trait RedundancyScheme: Send + Sync + 'static {
    /// Unique identifier in the database to denote how the object was handled.
    fn id(&self) -> &'static str;

    /// The number of shards that will be produced from a piece of data, up front.
    ///
    /// Replication is N, for N nodes.
    /// ECC(k, m) is k+m.
    ///
    /// The size of the return value of .shard()
    fn shard_count(&self) -> i32;

    /// A signal for optimization for the read path, or health for the repair path.
    ///
    /// Replication has 1, since all copies are identical
    /// ECC(k, m) is k, where any k shards are needed to reconstruct the data.
    fn min_shards_to_reconstruct(&self) -> i32;

    /// From a shard index, reconstruct the kind. It's important ShardKind::Data is preferred for GET and reading workloads, and ::Redundancy is usually for reconstruction.
    ///
    /// For replication, all shards are data shards.
    /// For ECC(k, m), indices from 0..(k-1) are Data and k..m-1 are Redundancy, since those are the parity shards.
    fn shard_kind(&self, shard_index: i32) -> ShardKind;

    /// Given a chunk of data in, split into shards that each should be persisted to
    /// distinct failure domains (other nodes).
    ///
    /// For replication, this means copying the identical data N times for N nodes.
    /// For ECC(k, m), this means encoding and returning k+m shards, k for data and m for parity.
    ///
    /// Always returns .shard_count() number of shards.
    ///
    /// Requirements:
    ///   data: must be memory-sized, ideally 1-4MB
    fn shard(&self, data: &Bytes) -> Result<Vec<Shard>, anyhow::Error>;

    /// Given shards read from distinct failure domains, recombine into the chunk of data.
    ///
    /// For replication this means take any of them, they are all identical.
    /// For ECC(k, m) that means combine the k data shards, or reconstruct.
    fn unshard(&self, shards: Vec<Shard>) -> Result<Bytes, anyhow::Error>;

    /// Given the set of available shards of a chunk of data, reconstruct the entire chunk.
    ///
    /// Replication, this just means copying one of the available pieces of data.
    /// For ECC(k, m), this means understanding what the index means and reconstructing the data or parity, depending.
    fn regenerate_shard(
        &self,
        missing_index: i32,
        available: &[Shard],
    ) -> Result<Bytes, anyhow::Error>;
}
