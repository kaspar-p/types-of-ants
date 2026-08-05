use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use rand::{Rng, rng};

use crate::AntArchiveStorageError;

/// Wraps `plaintext` as `nonce(12) || AES-256-GCM(plaintext, tek, nonce)`,
/// matching exactly what ant-archive-storage's put_blob expects to receive
/// and persists byte-for-byte unchanged.
pub(crate) fn wrap(tek: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AntArchiveStorageError> {
    let mut nonce = [0u8; 12];
    rng().fill_bytes(&mut nonce);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(tek));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| AntArchiveStorageError::Encryption(e.to_string()))?;

    let mut wire = Vec::with_capacity(12 + ciphertext.len());
    wire.extend_from_slice(&nonce);
    wire.extend_from_slice(&ciphertext);
    Ok(wire)
}

/// Reverses tek_wrap: splits the stored `nonce(12) || ciphertext` and decrypts.
pub(crate) fn unwrap(
    tek: &[u8; 32],
    storage_key: &str,
    wire: &[u8],
) -> Result<Vec<u8>, AntArchiveStorageError> {
    if wire.len() < 12 {
        return Err(AntArchiveStorageError::Decryption(format!(
            "{storage_key}: stored blob too short to contain TEK nonce"
        )));
    }
    let (nonce_bytes, ciphertext) = wire.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(tek));
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| AntArchiveStorageError::Decryption(storage_key.to_string()))
}
