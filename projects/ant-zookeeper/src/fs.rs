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

pub(crate) fn project_envs_file_name(project: &str, environment: &str) -> String {
    format!("{project}.{environment}.build.cfg")
}

pub(crate) fn global_envs_file_name(environment: &str) -> String {
    format!("{environment}.build.cfg")
}

pub(crate) fn secret_file_path(
    root_dir: &PathBuf,
    environment: &str,
    secret_name: &str,
) -> PathBuf {
    root_dir
        .join("secrets-db")
        .join(environment)
        .join(secret_file_name(secret_name))
}

pub(crate) fn secret_file_name(secret_name: &str) -> String {
    format!("{secret_name}.secret")
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
