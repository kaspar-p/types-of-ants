use std::{path::PathBuf, sync::Arc};

use acme_lib::DirectoryUrl;
use ant_zoo_storage::AntZooStorageClient;
use tokio::sync::Mutex;

use crate::dns::Dns;

#[derive(Clone)]
pub struct AntZookeeperState {
    pub root_dir: PathBuf,

    pub db: AntZooStorageClient,

    pub dns: Arc<Mutex<dyn Dns>>,
    pub acme_contact_email: String,
    pub acme_url: DirectoryUrl<'static>,

    pub rng: rsa::rand_core::OsRng,
}
