use std::{path::PathBuf, sync::Arc};

use ant_fs_client::AntFsClient;
use ant_library::sd::ServiceDiscovery;

use crate::storage_client::AntBackingItUpStorageClient;

#[derive(Clone)]
pub struct AntBackingItUpState {
    pub sd: Arc<ServiceDiscovery>,
    pub root_dir: PathBuf,
    pub ant_fs: AntFsClient,
    pub db: AntBackingItUpStorageClient,
}
