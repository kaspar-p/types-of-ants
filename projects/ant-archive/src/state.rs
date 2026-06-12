use std::sync::Arc;

use ant_archive_db::AntArchiveDb;

use crate::storage_client::AntArchiveStorageNodeClient;

#[derive(Clone)]
pub struct AntArchiveState {
    pub db: AntArchiveDb,
    pub storage_nodes: Vec<AntArchiveStorageNodeClient>,
    pub kek_id: String,
    kek: Arc<[u8; 32]>,
}

impl AntArchiveState {
    pub fn new(
        db: AntArchiveDb,
        storage_nodes: Vec<AntArchiveStorageNodeClient>,
        kek_id: String,
        kek: [u8; 32],
    ) -> Self {
        Self {
            db,
            storage_nodes,
            kek_id,
            kek: Arc::new(kek),
        }
    }

    pub fn kek(&self) -> &[u8; 32] {
        &self.kek
    }
}
