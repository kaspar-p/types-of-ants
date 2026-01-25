use std::path::PathBuf;
use std::{fs::File, io::Write};

use ant_library::headers::{XAntArchitectureHeader, XAntProjectHeader, XAntVersionHeader};
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use axum_extra::{routing::RouterExt, TypedHeader};
use flate2::read::GzDecoder;
use http::StatusCode;
use humansize::DECIMAL;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tempfile::tempdir_in;
use tokio::fs::create_dir_all;
use tracing::{info, warn};

use crate::fs::{artifact_file_name, artifact_persist_dir, envs_file_name, envs_persist_dir};
use crate::{err::AntZookeeperError, state::AntZookeeperState};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectKind {
    Docker,
    Binary {
        /// Defaults to the project's name
        binary_name_override: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct RegisterServiceRequest {
    pub project: String,
    pub kind: ProjectKind,
    pub is_owned: bool,
}

/// Register a new service if not already done
async fn register_service(
    State(state): State<AntZookeeperState>,
    Json(req): Json<RegisterServiceRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if state.db.get_project(&req.project).await? {
        return Ok((
            StatusCode::BAD_REQUEST,
            format!("Project {} already exists", req.project),
        ));
    }

    state
        .db
        .register_project(&req.project, req.is_owned)
        .await?;

    Ok((StatusCode::OK, "Project registered.".to_string()))
}

pub fn binary_systemd_unit(
    project: &str,
    project_description: &str,
    install_dir: &PathBuf,
) -> String {
    let install_dir = install_dir.to_str().unwrap();
    format!(
        "[Unit]
Description={project_description}

[Service]
Type=simple
EnvironmentFile={install_dir}/.env
Environment=TYPESOFANTS_SECRET_DIR={install_dir}/secrets
ExecStart={install_dir}/{project}
WorkingDirectory={install_dir}
Restart=always

[Install]
WantedBy=multi-user.target
"
    )
}

pub fn docker_systemd_unit(
    project: &str,
    project_description: &str,
    install_dir: &PathBuf,
) -> String {
    let install_dir = install_dir.to_str().unwrap();

    format!("[Unit]
Description={project_description}

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory={install_dir} up --no-build --force-recreate {project}
ExecStop=/snap/bin/docker-compose --project-directory={install_dir} stop {project}
EnvironmentFile={install_dir}/.env
WorkingDirectory={install_dir}
Restart=always

[Install]
WantedBy=multi-user.target
")
}

/// An API to ingest a new build artifact, a new built version of a service for a given platform.
async fn register_artifact(
    State(state): State<AntZookeeperState>,
    TypedHeader(project): TypedHeader<XAntProjectHeader>,
    TypedHeader(arch): TypedHeader<XAntArchitectureHeader>,
    TypedHeader(version): TypedHeader<XAntVersionHeader>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.get_project(&project.0).await? {
        info!("Registering project [{}] for the first time...", project.0);
        let is_owned = true; // Building our own images means an owned project!
        state.db.register_project(&project.0, is_owned).await?;
    }

    let global_tmp_dir = state.root_dir.join("tmp");
    create_dir_all(&global_tmp_dir).await?;

    let dir = artifact_persist_dir(&state.root_dir);
    create_dir_all(&dir).await?;

    let temp_dir = tempdir_in(&global_tmp_dir)?;
    let temp_file_path = temp_dir.path().join("input.tar.gz");
    let mut temp_file = File::create_new(&temp_file_path)?;
    {
        let mut field = multipart
            .next_field()
            .await
            .map_err(|e| {
                warn!("No field in multipart: {e}");
                AntZookeeperError::validation_msg("No field found in multipart request!")
            })?
            .ok_or(AntZookeeperError::validation_msg(
                "No bytes field found in request!",
            ))?;

        while let Some(bytes) = field.chunk().await.unwrap() {
            info!(
                "Wrote [{}] to [{}]...",
                humansize::format_size(bytes.len(), DECIMAL),
                temp_file_path.display()
            );
            temp_file.write_all(&bytes)?;
        }
        temp_file.flush()?;
        info!("Finished writing to [{}]...", temp_file_path.display());
    }

    info!(
        "Validating file contents of [{}]...",
        temp_file_path.display()
    );

    {
        let temp_file = File::open(&temp_file_path)?;
        let gz = GzDecoder::new(&temp_file);
        let mut archive = Archive::new(gz);

        let mut service_file_found = false;

        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?;
            if path.is_dir() {
                continue;
            }

            let file_name = path
                .file_name()
                .expect(format!("entry {} has file name", path.display()).as_str());

            if file_name == ".env" {
                return Err(AntZookeeperError::validation_msg(
                    "Deployment tarball cannot contain any files named '.env'.",
                ));
            }

            if file_name == format!("{}.service", project.0).as_str() {
                service_file_found = true;
            }
        }

        if !service_file_found {
            return Err(AntZookeeperError::validation_msg(
                format!(
                    "Service file '{}.service' must be included in deployment tarball.",
                    project.0
                )
                .as_str(),
            ));
        }
    }

    // Write the file to final location
    let filepath = dir.join(artifact_file_name(&project.0, arch.0.as_ref(), &version.0));
    {
        info!("Writing tarball to [{}]", filepath.display());
        std::fs::copy(&temp_file_path, &filepath)?;
    }

    let revision_id = state.db.upsert_revision(&project.0, &version.0).await?;

    info!("Registering or updating artifact...");
    let artifact_id = state
        .db
        .get_artifact_by_revision(arch.0.as_ref(), &revision_id)
        .await?;

    match artifact_id {
        Some((artifact_id, _, _)) => {
            info!("Updating existing artifact");
            state.db.update_artifact(&artifact_id).await?;
        }
        None => {
            info!("Registering new artifact");
            let relative = filepath.strip_prefix(&dir).expect(&format!(
                "[{}] was not parent of [{}]",
                dir.display(),
                filepath.display()
            ));
            state
                .db
                .register_artifact(&revision_id, arch.0.as_ref(), &relative)
                .await?;
        }
    }

    info!("Determining pipeline promotions...");
    // if all architectures are done for a given (project, version), then we are done with the 'build' stage in a pipeline.
    let missing_architectures = state
        .db
        .missing_artifacts_for_project_version(&project.0, &version.0)
        .await?;
    if missing_architectures.is_empty() {
        info!("All architectures for a project have been built, build stage fulfilled.");
    } else {
        info!("Still missing architectures: {missing_architectures:?}");
    }

    Ok((StatusCode::OK, "Version registered"))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentVariable {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutProjectEnvironmentRequest {
    pub project: String,
    pub environment: String,
    pub variables: Vec<ProjectEnvironmentVariable>,
}

async fn put_project_environment(
    State(state): State<AntZookeeperState>,
    Json(req): Json<PutProjectEnvironmentRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    create_dir_all(envs_persist_dir(&state.root_dir)).await?;
    let envs_file_path =
        envs_persist_dir(&state.root_dir).join(envs_file_name(&req.project, &req.environment));

    let mut file = File::create(envs_file_path)?;

    for variable in req.variables {
        let text = format!("{}=\"{}\"\n", variable.key, variable.value);
        file.write_all(text.as_bytes())?
    }

    Ok((StatusCode::OK, "Project environment registered"))
}

// #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct GetProjectEnvironmentRequest {
//     pub project: String,
//     pub environment: String,
// }

// // #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct GetProjectEnvironmentResponse {
//     pub variables: Vec<ProjectEnvironmentVariable>,
// }
// async fn get_project_environment(
//     State(state): State<AntZookeeperState>,
//     Json(req): Json<GetProjectEnvironmentRequest>,
// ) -> Result<impl IntoResponse, AntZookeeperError> {
//     let envs_file_path =
//         envs_persist_dir(&state.root_dir).join(envs_file_name(&req.project, &req.environment));

//     Ok((StatusCode::OK, Json(GetProjectEnvironmentResponse{variables})))
// }

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new()
        .route("/service", post(register_service))
        .route("/env", post(put_project_environment))
        .route_with_tsr(
            "/artifact",
            post(register_artifact).layer(
                DefaultBodyLimit::max(1000 * 1000 * 1000), // 1GB
            ),
        )
}
