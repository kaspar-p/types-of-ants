use std::sync::Arc;

use ant_archive_db::AntArchiveDb;
use ant_library::sd::reader::ServiceDiscovery;

#[derive(Clone)]
pub struct AntArchiveState {
    pub db: AntArchiveDb,
    pub sd: Arc<ServiceDiscovery>,
}

impl AntArchiveState {
    pub fn new(db: AntArchiveDb, sd: Arc<ServiceDiscovery>) -> Self {
        Self { db, sd }
    }
}
