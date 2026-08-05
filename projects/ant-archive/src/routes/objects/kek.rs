use aes_gcm::{aead::Aead, AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use base64ct::{Base64, Encoding};
use rand::rngs::OsRng;

use crate::AntArchiveError;

pub(super) fn load_kek(kek_id: &str, kek_alias: Option<&str>) -> Result<[u8; 32], AntArchiveError> {
    // ant_archive_kek contains one entry per line: "{kek_alias}:{base64(32 bytes)}"
    let content = ant_library::secret::load_secret("ant_archive_kek")?;
    for line in content.lines() {
        let Some((id, b64)) = line.split_once(':') else {
            continue;
        };

        match kek_alias {
            Some(kek_alias) if id != kek_id && id != kek_alias => {
                continue;
            }
            _ => {}
        }

        let bytes = Base64::decode_vec(b64).map_err(|e| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-091",
                Some(anyhow::anyhow!(
                    "ant_archive_kek entry for '{kek_alias:?}' or '{kek_id}' is not valid base64: {e}"
                )),
            )
        })?;
        let len = bytes.len();
        return bytes.try_into().map_err(|_| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-092",
                Some(anyhow::anyhow!(
                    "ant_archive_kek entry for '{kek_alias:?}' or '{kek_id}' must be exactly 32 bytes, got {len}"
                )),
            )
        });
    }
    Err(AntArchiveError::InternalServerError(
        "ANT-ERR-093",
        Some(anyhow::anyhow!(
            "ant_archive_kek has no entry for kek_id '{kek_alias:?}' or '{kek_id}'"
        )),
    ))
}

pub(super) struct EncryptedDek {
    pub dek_nonce: Vec<u8>,
    pub dek_ciphertext: Vec<u8>,
}

pub(super) fn encrypt_dek(kek: &[u8], dek: &[u8]) -> Result<EncryptedDek, AntArchiveError> {
    let dek_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let kek_key = Key::<Aes256Gcm>::from_slice(kek);
    let kek_cipher = Aes256Gcm::new(kek_key);
    let encrypted_dek = kek_cipher.encrypt(&dek_nonce, dek.as_ref()).map_err(|e| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-097",
            Some(anyhow::anyhow!("DEK encryption failed: {e}")),
        )
    })?;

    Ok(EncryptedDek {
        dek_ciphertext: encrypted_dek,
        dek_nonce: dek_nonce.into_iter().collect(),
    })
}

pub(super) fn decrypt_dek(
    kek: &[u8],
    encrypted_dek: &EncryptedDek,
) -> Result<[u8; 32], AntArchiveError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(kek));
    let nonce = Nonce::from_slice(&encrypted_dek.dek_nonce);

    let dek = cipher
        .decrypt(nonce, encrypted_dek.dek_ciphertext.as_ref())
        .map_err(|e| {
            AntArchiveError::InternalServerError(
                "ANT-ERR-102",
                Some(anyhow::anyhow!("DEK decryption failed: {e}")),
            )
        })?;

    let dek_len = dek.len();
    dek.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-103",
            Some(anyhow::anyhow!(
                "DEK wrong length: expected 32 bytes, got {dek_len}"
            )),
        )
    })
}
