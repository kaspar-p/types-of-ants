use std::path::PathBuf;

#[derive(Clone)]
pub struct AntHostAgentState {
    /// Where secrets that this ant-host-agent service (and other services via replication) use.
    pub secrets_root_dir: PathBuf,

    /// Where to save temporary files or find deployment archive files, as the input.
    pub archive_root_dir: PathBuf,

    /// The destination of installation files, after unpacking.
    pub install_root_dir: PathBuf,
}
