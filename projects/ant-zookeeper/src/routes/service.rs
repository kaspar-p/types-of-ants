use std::path::PathBuf;

use ant_zoo_storage::host_architecture::HostArchitecture;
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use axum_extra::{routing::RouterExt, TypedHeader};
use http::StatusCode;
use humansize::DECIMAL;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};
use tracing::info;

use crate::{
    err::AntZookeeperError,
    routes::headers::{XAntArchitectureHeader, XAntProjectHeader, XAntVersionHeader},
    state::AntZookeeperState,
};

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

    let dir = persist_dir(&state.root_dir);
    create_dir_all(&dir).await?;

    let path = dir.join(service_file_name(&project.0, arch.0.as_ref(), &version.0));
    let mut file = File::create(&path).await?;

    let mut field = multipart
        .next_field()
        .await
        .map_err(|e| {
            AntZookeeperError::validation("No field found in multipart request!", Some(e.into()))
        })?
        .ok_or(AntZookeeperError::validation(
            "No bytes field found in request!",
            None,
        ))?;

    while let Some(bytes) = field.chunk().await.unwrap() {
        info!(
            "Wrote [{}] to [{}]...",
            humansize::format_size(bytes.len(), DECIMAL),
            path.display()
        );
        file.write_all(&bytes).await?;
    }
    file.flush().await?;

    info!("Finished writing to [{}]...", path.display());

    if state
        .db
        .artifact_exists(&project.0, arch.0.as_ref(), &version.0)
        .await?
    {
        info!("Updating existing artifact");
        state
            .db
            .update_artifact(&project.0, arch.0.as_ref(), &version.0)
            .await?;
    } else {
        info!("Registering new artifact");
        let relative = path.strip_prefix(&dir).expect(&format!(
            "[{}] was not parent of [{}]",
            dir.display(),
            path.display()
        ));
        state
            .db
            .register_artifact(&project.0, arch.0.as_ref(), &version.0, &relative)
            .await?;
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
