use std::path::PathBuf;

use ant_library::host_architecture::HostArchitecture;

pub(crate) fn artifact_persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("artifacts-db")
}

pub(crate) fn artifact_file_name(
    project: &str,
    arch: Option<&HostArchitecture>,
    version: &str,
) -> String {
    format!(
        "{}.{}.{}.bld",
        project,
        arch.map(|a| a.as_str()).unwrap_or("noarch").to_string(),
        version
    )
}

pub(crate) fn envs_persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("envs")
}

pub(crate) fn envs_file_name(project: &str, environment: &str) -> String {
    format!("{project}.{environment}.build.cfg")
}

pub(crate) fn services_persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("services-db")
}

pub(crate) fn services_file_name(
    project: &str,
    arch: Option<&HostArchitecture>,
    version: &str,
) -> String {
    format!("{}.deployable", artifact_file_name(project, arch, version))
}
