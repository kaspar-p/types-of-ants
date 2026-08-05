use bytes::Bytes;
use futures::stream::BoxStream;

#[derive(Debug, thiserror::Error)]
pub enum ChunkError {
    #[error("encryption failed: {0}")]
    Crypto(#[from] aes_gcm::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A chunk of the input data stream that has been independently encrypted.
pub struct EncryptedChunk {
    pub index: u64,
    pub is_last_chunk: bool,
    pub ciphertext: Bytes,
    pub plaintext_len: usize,
}

pub trait Chunker: Send + Sync + 'static {
    /// Stable string id, e.g. "whole", "fixed:4194304" — persisted on the object row.
    fn id(&self) -> &'static str;

    /// Nominal plaintext bytes per chunk. usize::MAX for "whole blob, one chunk"
    /// (this is how v1 objects compose into the same trait, no special-casing).
    fn chunk_size(&self) -> usize;

    /// Reads plaintext, encrypts it into a stream of chunks.
    /// Handles nonce derivation and last-chunk lookahead internally.
    fn encrypt_stream(
        &self,
        dek: [u8; 32],
        nonce_prefix: [u8; 4],
        plaintext: BoxStream<'static, std::io::Result<Bytes>>,
    ) -> BoxStream<'static, Result<EncryptedChunk, ChunkError>>;

    /// Decrypts one chunk's ciphertext. chunk_idx/is_last_chunk are supplied
    /// by the caller (derived from chunk_idx == object.chunk_count - 1) since
    /// this method reconstructs one chunk at a time.
    fn decrypt_chunk(
        &self,
        dek: &[u8; 32],
        nonce_prefix: &[u8; 4],
        chunk_idx: u64,
        is_last_chunk: bool,
        ciphertext: Bytes,
    ) -> Result<Bytes, ChunkError>;
}
