use std::path::PathBuf;
use std::{fs::File, io::Write};

use ant_library::{
    headers::{XAntArchitectureHeader, XAntProjectHeader, XAntVersionHeader},
    host_architecture::HostArchitecture,
};
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
use tracing::info;

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

fn persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("services-db")
}

fn service_file_name(project: &str, arch: Option<&HostArchitecture>, version: &str) -> String {
    format!(
        "{}.{}.{}.bld",
        project,
        arch.map(|a| a.as_str()).unwrap_or("noarch").to_string(),
        version
    )
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

    let dir = persist_dir(&state.root_dir);
    create_dir_all(&dir).await?;

    let temp_dir = tempdir_in(&global_tmp_dir)?;
    let temp_file_path = temp_dir.path().join("input.tar.gz");
    let mut temp_file = File::create_new(&temp_file_path)?;
    {
        let mut field = multipart
            .next_field()
            .await
            .map_err(|e| {
                AntZookeeperError::validation(
                    "No field found in multipart request!",
                    Some(e.into()),
                )
            })?
            .ok_or(AntZookeeperError::validation(
                "No bytes field found in request!",
                None,
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
    let filepath = dir.join(service_file_name(&project.0, arch.0.as_ref(), &version.0));
    {
        info!("Writing tarball to [{}]", filepath.display());
        std::fs::copy(&temp_file_path, &filepath)?;
    }

    info!("Registering or updating artifact...");
    let artifact_id = state
        .db
        .artifact_exists(&project.0, arch.0.as_ref(), &version.0)
        .await?;

    match artifact_id {
        Some(artifact_id) => {
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
                .register_artifact(&project.0, arch.0.as_ref(), &version.0, &relative)
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
        let build_stage_id = state
            .db
            .get_deployment_pipeline_build_stage(&project.0)
            .await?;

        state
            .db
            .make_deployment(&build_stage_id, &project.0, &version.0)
            .await?;
    } else {
        info!("Still missing architectures: {missing_architectures:?}");
    }

    Ok((StatusCode::OK, "Version registered"))
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new()
        .route("/service", post(register_service))
        .route_with_tsr(
            "/artifact",
            post(register_artifact).layer(
                DefaultBodyLimit::max(1000 * 1000 * 1000), // 1GB
            ),
        )
}
