use std::{collections::HashMap, path::PathBuf, sync::Arc};

use ant_library::sd::writer::ServiceDiscoveryWriter;
use anthill_manifest::AnthillManifest;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct AntHostAgentState {
    /// A mini-database for keeping track of the services present on this host.
    /// Filled on startup and as services are enabled/disabled.
    ///
    /// Keys are service IDs ("ant-host-agent", or "ant-db-metrics.ant-data-farm")
    pub services: Arc<Mutex<HashMap<String, HostService>>>,

    pub sd: Arc<ServiceDiscoveryWriter>,

    /// Where secrets that this ant-host-agent service (and other services via replication) use.
    pub secrets_root_dir: PathBuf,

    /// Where to save temporary files or find deployment archive files, as the input.
    ///
    /// This directory belongs to the ant-host-agent process
    pub archive_root_dir: PathBuf,

    /// The destination of installation files, after unpacking.
    ///
    /// This directory DOES NOT belong to ant-host-agent, be careful with it!
    pub install_root_dir: PathBuf,
}

#[derive(Debug)]
pub struct HostService {
    pub manifest: AnthillManifest,
}
