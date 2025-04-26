use std::{fs::File, path::PathBuf, process::Command, sync::Arc};

use ant_data_farm::AntDataFarmClient;
use axum::{extract::State, response::IntoResponse, Json};
use flate2::{write::GzEncoder, Compression};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

fn artifact_path(project_version: &str) -> String {
    return format!("./repos/{project_version}");
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MakeInput {
    pub project_id: String,
    pub project_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MakeOutput {
    pub project_id: String,
    pub project_version: String,
}

pub async fn make(
    State(db): State<Arc<AntDataFarmClient>>,
    Json(input): Json<MakeInput>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let path = PathBuf::from(artifact_path(&input.project_version));

    // Clone the repository if not already cloned
    let repo = if path.exists() && path.is_dir() && path.join(".git").is_dir() {
        git2::Repository::open(path.clone()).expect("Failed to open repository.")
    } else {
        git2::Repository::clone("https://github.com/kaspar-p/types-of-ants", path.clone())
            .expect("Failed to clone repository.")
    };

    // Try to parse the version (commit SHA) to see if it's valid.
    let (object, reference) = match repo.revparse_ext(&input.project_version) {
        Ok(v) => v,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid version '{}'", input.project_version),
            ))
        }
    };

    // Checkout the version
    repo.checkout_tree(&object, None)
        .expect("Failed to checkout version");
    match reference {
        // ref is an actual reference like branches or tags
        Some(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Version must be commit hash.".to_string(),
            ))
        }
        // Ref is a commit
        None => {
            repo.set_head_detached(object.id())
                .expect("Failed to set HEAD to version.");
        }
    }

    match Command::new("make")
        .current_dir(path.clone().join("projects").join(input.project_id.clone()))
        .arg("release")
        .output()
    {
        Err(e) => {
            error!("Build failure: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Build failure".to_string(),
            ));
        }
        Ok(stdout) => {
            debug!("Build output: \n{:?}", stdout);
        }
    }

    // TODO: use some sort of manifest to convert a "known" project type
    // into a deployable artifact, after the compilation is finished.

    let tarfile_path = path
        .clone()
        .parent()
        .unwrap()
        .join(input.project_version.clone())
        .join("artifact.tar.gz");
    let tarfile = match File::create(tarfile_path) {
        Err(e) => {
            error!("Failed to create tarfile: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create tarfile".to_string(),
            ));
        }
        Ok(f) => f,
    };
    let encoder = GzEncoder::new(tarfile, Compression::default());
    let mut tar = tar::Builder::new(encoder);

    let build_dir = path
        .clone()
        .join("projects")
        .join(input.project_id.clone())
        .join("build");
    match tar.append_dir_all("build", build_dir.clone()) {
        Err(e) => {
            error!(
                "Failed to append build dir {:?} to tar file: {}",
                build_dir.clone(),
                e
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to append build dir to tar file".to_string(),
            ));
        }
        Ok(()) => (),
    }
    match tar.finish() {
        Err(e) => {
            error!("Failed to write to tarfile: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to write tarfile".to_string(),
            ));
        }
        Ok(_) => (),
    }

    return Ok((
        StatusCode::OK,
        Json(MakeOutput {
            project_id: input.project_id.clone(),
            project_version: input.project_version.clone(),
        })
        .into_response(),
    ));
}
