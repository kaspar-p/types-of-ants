use std::path::PathBuf;

use ant_fs_client::AntFsClient;

use crate::storage_client::AntBackingItUpStorageClient;

#[derive(Clone)]
pub struct AntBackingItUpState {
    pub root_dir: PathBuf,
    pub ant_fs: AntFsClient,
    pub db: AntBackingItUpStorageClient,
}
