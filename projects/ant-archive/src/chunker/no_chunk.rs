use bytes::{Bytes, BytesMut};
use futures::{stream::BoxStream, StreamExt};

use crate::chunker::{
    chunker::{ChunkError, Chunker, EncryptedChunk},
    crypto::{aead_decrypt, aead_encrypt, chunk_nonce},
};

pub struct NoChunk {}

impl Chunker for NoChunk {
    fn id(&self) -> &'static str {
        "no_chunk"
    }

    fn chunk_size(&self) -> usize {
        usize::MAX
    }

    fn encrypt_stream(
        &self,
        dek: [u8; 32],
        nonce_prefix: [u8; 4],
        mut plaintext: BoxStream<'static, std::io::Result<Bytes>>,
    ) -> BoxStream<'static, Result<EncryptedChunk, ChunkError>> {
        Box::pin(futures::stream::once(async move {
            let entire_plaintext = {
                let mut buffer = BytesMut::new();

                while let Some(chunk) = plaintext.next().await {
                    let chunk = chunk?;
                    buffer.extend_from_slice(&chunk);
                }

                buffer.freeze()
            };

            let ciphertext = aead_encrypt(
                &dek,
                &chunk_nonce(&nonce_prefix, 0),
                b"last",
                &entire_plaintext,
            )?;

            Ok(EncryptedChunk {
                index: 0,
                is_last_chunk: true,
                ciphertext: ciphertext.into(),
                plaintext_len: entire_plaintext.len(),
            })
        }))
    }

    fn decrypt_chunk(
        &self,
        dek: &[u8; 32],
        nonce_prefix: &[u8; 4],
        chunk_idx: u64,
        is_last: bool,
        ciphertext: Bytes,
    ) -> Result<Bytes, ChunkError> {
        let nonce = chunk_nonce(nonce_prefix, chunk_idx);

        let bytes = aead_decrypt(
            dek,
            &nonce,
            if is_last { b"last" } else { b"cont" },
            &ciphertext,
        )?;

        Ok(Bytes::from(bytes))
    }
}
