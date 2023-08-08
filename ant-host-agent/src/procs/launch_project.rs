use crate::common::launch_project::{LaunchProjectResponse, LaunchStatus};
use ant_metadata::{get_typesofants_home, Project};
use anyhow::Result;
use hyper::body::Bytes;
use std::io::Write;
use std::path::PathBuf;
use sysinfo::{Process, System, SystemExt};
use tracing::debug;

fn already_running(project: Project) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();

    let exists = sys
        .processes_by_exact_name(project.as_str())
        .collect::<Vec<&Process>>()
        .len()
        > 0;
    return exists;
}

fn artifact_dir_path(project: Project) -> PathBuf {
    let mut path = get_typesofants_home();
    path.push("run");
    path.push(project.as_str());
    return path;
}

async fn save_artifact(project: Project, artifact: Bytes) -> Result<PathBuf> {
    // Save
    let path = artifact_dir_path(project);
    let parent = path.parent().ok_or(anyhow::Error::msg("No parent dir!"))?;
    std::fs::create_dir_all(parent)?;

    let mut f = std::fs::File::create(&path)?;
    f.write_all(&artifact)?;

    // Unpack
    let mut archive = tar::Archive::new(f);
    archive.unpack(project.as_str())?;

    // Remove original artifact
    std::fs::remove_file(&path)?;

    return Ok(path);
}

pub enum LaunchProjectError {
    AlreadyExists,
    LaunchBinary(anyhow::Error),
    SaveArtifact(anyhow::Error),
}

pub async fn launch_project(
    project: Project,
    artifact: Bytes,
) -> Result<LaunchProjectResponse, LaunchProjectError> {
    if already_running(project) {
        return Err(LaunchProjectError::AlreadyExists);
    }

    let dir = match save_artifact(project, artifact).await {
        Err(e) => return Err(LaunchProjectError::SaveArtifact(e)),
        Ok(b) => b,
    };

    // Assume there's a top-level launch.sh startup script in the binary.
    let launch_script = dir.join("launch.sh");

    match std::process::Command::new(launch_script).spawn() {
        Err(e) => {
            debug!("Error launching: {e}");
            return Err(LaunchProjectError::LaunchBinary(e.into()));
        }
        Ok(child) => {
            debug!("Launched with pid {}", child.id());
        }
    }

    // curl -F ‘data=@path/to/local/file’ UPLOAD_ADDRESS

    return Ok(LaunchProjectResponse {
        status: LaunchStatus::LaunchSuccessful,
    });
}
