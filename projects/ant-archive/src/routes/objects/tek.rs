use hkdf::Hkdf;
use sha2::Sha256;

use crate::AntArchiveError;

pub(super) fn load_tek_master() -> Result<[u8; 32], AntArchiveError> {
    let bytes = ant_library::secret::load_secret_binary("ant_archive_tek")?;
    let len = bytes.len();
    bytes.try_into().map_err(|_| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-094",
            Some(anyhow::anyhow!("tek must be exactly 32 bytes, got {len}")),
        )
    })
}

pub(super) fn derive_tek(tek_derivation_key: &[u8]) -> Result<[u8; 32], AntArchiveError> {
    let tek_master = load_tek_master()?;

    let hkdf = Hkdf::<Sha256>::new(None, &tek_master);
    let mut tek = [0u8; 32];
    hkdf.expand(tek_derivation_key, &mut tek).map_err(|e| {
        AntArchiveError::InternalServerError(
            "ANT-ERR-095",
            Some(anyhow::anyhow!("TEK derivation failed: {e}")),
        )
    })?;
    Ok(tek)
}
