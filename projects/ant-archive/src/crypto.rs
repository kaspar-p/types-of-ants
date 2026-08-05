use sha2::{Digest, Sha256};

pub(super) fn generate_random_32(rng: &dyn ant_library::rng::Rng) -> [u8; 32] {
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    key
}

pub(super) fn generate_random_4(rng: &dyn ant_library::rng::Rng) -> [u8; 4] {
    let mut key = [0u8; 4];
    rng.fill(&mut key);
    key
}

pub(crate) fn compute_checksum(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).into_iter().collect()
}
