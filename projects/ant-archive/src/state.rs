use std::sync::Arc;

use ant_archive_db::AntArchiveDb;
use ant_library::{rng::Rng, sd::reader::ServiceDiscovery};

#[derive(Clone)]
pub struct AntArchiveState {
    pub db: AntArchiveDb,
    pub sd: Arc<ServiceDiscovery>,
    pub rng: Arc<dyn Rng>,
}
