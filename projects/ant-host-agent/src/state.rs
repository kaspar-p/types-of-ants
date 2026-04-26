use std::path::PathBuf;

#[derive(Clone)]
pub struct AntHostAgentState {
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
