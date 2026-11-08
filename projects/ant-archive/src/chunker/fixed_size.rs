use bytes::Bytes;
use futures::stream::{self, BoxStream};
use tokio_util::io::StreamReader;

use crate::chunker::{
    chunker::{ChunkError, Chunker, EncryptedChunk},
    crypto,
};

pub struct FixedSize {
    pub size: usize, // e.g. 4 * 1024 * 1024
}

struct FixedSizeState {
    reader: StreamReader<BoxStream<'static, std::io::Result<Bytes>>, Bytes>,
    dek: [u8; 32],
    nonce_prefix: [u8; 4],
    chunk_size: usize,
    idx: u64,
    pending: Option<(Vec<u8>, usize)>, // buffer + filled length, not yet emitted
    finished: bool,
}

impl FixedSize {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

impl Chunker for FixedSize {
    fn id(&self) -> &'static str {
        "fixed_size"
    }

    fn chunk_size(&self) -> usize {
        self.size
    }

    fn ciphertext_sizes(&self, plaintext_len: usize) -> Vec<usize> {
        // A zero-byte body is still one (empty) chunk.
        if plaintext_len == 0 {
            return vec![crypto::TAG_SIZE];
        }

        let full_chunks = plaintext_len / self.size;
        let remainder = plaintext_len % self.size;

        let mut sizes = vec![self.size + crypto::TAG_SIZE; full_chunks];
        if remainder > 0 {
            sizes.push(remainder + crypto::TAG_SIZE);
        }
        sizes
    }

    fn encrypt_stream(
        &self,
        dek: [u8; 32],
        nonce_prefix: [u8; 4],
        plaintext: BoxStream<'static, std::io::Result<Bytes>>,
    ) -> BoxStream<'static, Result<EncryptedChunk, ChunkError>> {
        let initial = FixedSizeState {
            reader: StreamReader::new(plaintext),
            dek,
            nonce_prefix,
            chunk_size: self.size,
            idx: 0,
            pending: None,
            finished: false,
        };

        Box::pin(stream::unfold(initial, |mut state| async move {
            if state.finished {
                return None;
            }

            // First call only: prime `pending` with the first chunk read.
            if state.pending.is_none() {
                let mut buf = vec![0u8; state.chunk_size];
                let filled = match crypto::fill_fully(&mut state.reader, &mut buf).await {
                    Ok(n) => n,
                    Err(e) => {
                        state.finished = true;
                        return Some((Err(ChunkError::Io(e)), state));
                    }
                };
                if filled == 0 {
                    // Zero-byte body: emit a single empty last chunk.
                    state.finished = true;
                    let nonce = crypto::chunk_nonce(&state.nonce_prefix, state.idx);
                    let result: Result<EncryptedChunk, ChunkError> =
                        crypto::aead_encrypt(&state.dek, &nonce, b"last", &[])
                            .map_err(|e| e.into())
                            .map(|ct| EncryptedChunk {
                                index: state.idx,
                                is_last_chunk: true,
                                ciphertext: ct.into(),
                                plaintext_len: filled,
                            });

                    return Some((result, state));
                }
                state.pending = Some((buf, filled));
            }

            // Look one chunk ahead so we know whether `pending` is the last one —
            // this is the truncation-attack defense: the AAD domain-separates
            // "cont" vs "last" chunks, so a truncated upload fails to decrypt
            // as complete rather than silently looking like a shorter object.
            let mut next_buf = vec![0u8; state.chunk_size];
            let next_filled = match crypto::fill_fully(&mut state.reader, &mut next_buf).await {
                Ok(n) => n,
                Err(e) => {
                    state.finished = true;
                    return Some((Err(e.into()), state));
                }
            };
            let is_last = next_filled == 0;
            let aad: &[u8] = if is_last { b"last" } else { b"cont" };

            let (buf, filled) = state.pending.take().expect("pending primed above");
            let nonce = crypto::chunk_nonce(&state.nonce_prefix, state.idx);
            let result = crypto::aead_encrypt(&state.dek, &nonce, aad, &buf[..filled])
                .map_err(|e| e.into())
                .map(|ct| EncryptedChunk {
                    index: state.idx,
                    is_last_chunk: is_last,
                    ciphertext: ct.into(),
                    plaintext_len: filled,
                });

            if is_last {
                state.finished = true;
            } else {
                state.idx += 1;
                state.pending = Some((next_buf, next_filled));
            }
            Some((result, state))
        }))
    }

    fn decrypt_chunk(
        &self,
        dek: &[u8; 32],
        nonce_prefix: &[u8; 4],
        chunk_idx: u64,
        is_last_chunk: bool,
        ciphertext: Bytes,
    ) -> Result<Bytes, ChunkError> {
        let nonce = crypto::chunk_nonce(nonce_prefix, chunk_idx);
        let aad: &[u8] = if is_last_chunk { b"last" } else { b"cont" };
        Ok(crypto::aead_decrypt(dek, &nonce, aad, &ciphertext)?.into())
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;
    use futures::StreamExt;

    use crate::chunker::{chunker::Chunker, fixed_size::FixedSize};

    #[tokio::test]
    async fn ciphertext_sizes_match_encrypt_stream() {
        let chunker = FixedSize::new(10);

        for plaintext_len in [0, 1, 9, 10, 11, 20, 25] {
            let plaintext = Bytes::from(vec![7u8; plaintext_len]);
            let stream = futures::stream::once(async move { Ok(plaintext) }).boxed();

            let actual: Vec<usize> = chunker
                .encrypt_stream([1u8; 32], [2u8; 4], stream)
                .map(|chunk| chunk.unwrap().ciphertext.len())
                .collect()
                .await;

            assert_eq!(
                chunker.ciphertext_sizes(plaintext_len),
                actual,
                "plaintext of {plaintext_len} bytes"
            );
        }
    }
}
