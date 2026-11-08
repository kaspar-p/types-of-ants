use std::sync::Arc;

use ant_archive_db::AntArchiveDb;
use ant_library::{rng::Rng, sd::reader::ServiceDiscovery};

#[derive(Clone)]
pub struct AntArchiveState {
    pub db: AntArchiveDb,
    pub sd: Arc<ServiceDiscovery>,
    pub rng: Arc<dyn Rng>,

    pub chunk_size: usize,

    /// How long an object stays on storage nodes after no key points to it, so uploads and
    /// reads in progress aren't cut off.
    pub vacuum_grace: chrono::Duration,
    /// Unique to this process, to hold leases on jobs shared with other instances.
    pub instance_id: String,
}
