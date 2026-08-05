use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, Key, KeyInit, Nonce,
};
use tokio::io::AsyncReadExt;

pub(super) fn chunk_nonce(prefix: &[u8; 4], index: u64) -> [u8; 12] {
    let mut n = [0u8; 12];
    n[..4].copy_from_slice(prefix);
    n[4..].copy_from_slice(&index.to_be_bytes());
    n
}

pub(super) fn aead_encrypt(
    dek: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(dek));
    let ciphertext = cipher.encrypt(
        Nonce::from_slice(nonce),
        Payload {
            msg: plaintext,
            aad,
        },
    )?;

    Ok(ciphertext)
}

pub(super) fn aead_decrypt(
    dek: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(dek));
    let plaintext = cipher.decrypt(
        Nonce::from_slice(nonce),
        Payload {
            msg: ciphertext,
            aad,
        },
    )?;

    Ok(plaintext)
}

/// Reads up to `buf.len()` bytes, stopping early only at EOF.
/// Returns the number of bytes actually filled.
pub(super) async fn fill_fully<R: tokio::io::AsyncRead + Unpin>(
    r: &mut R,
    buf: &mut [u8],
) -> std::io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]).await? {
            0 => break, // EOF — return what we have, however much that is
            read => n += read,
        }
    }
    Ok(n)
}
