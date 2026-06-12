use std::{path::PathBuf, sync::Arc};

use ant_library::sd::writer::ServiceDiscoveryWriter;

#[derive(Debug, Clone)]
pub struct AntHostAgentState {
    pub sd: Arc<ServiceDiscoveryWriter>,

    /// Where to save temporary files or find deployment archive files, as the input.
    ///
    /// This directory belongs to the ant-host-agent process
    pub archive_root_dir: PathBuf,

    /// The destination of installation files, after unpacking.
    ///
    /// This directory DOES NOT belong to ant-host-agent, be careful with it!
    pub install_root_dir: PathBuf,
}
