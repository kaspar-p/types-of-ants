use std::{io::ErrorKind, path::PathBuf, time::Duration};
use tokio_util::codec;

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use futures_util::stream::StreamExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, time::sleep};
use tracing::{error, info, warn};
use zbus_systemd::{systemd1::ManagerProxy, zbus};

use std::default::Default;

use crate::state::AntHostAgentState;

fn unit_name(project: &str) -> String {
    format!("{project}.service")
}

fn unit_file_path(project: &str, version: &str) -> String {
    format!(
        "/home/ant/service/{}/{}/{}",
        project,
        version,
        unit_name(project)
    )
}

#[derive(Serialize, Deserialize)]
pub struct EnableServiceRequest {
    pub project: String,
    pub version: String,
}

async fn enable_unit(manager: &ManagerProxy<'_>, project: &str, version: &str) {
    info!("Enabling service...");
    let unit_file_path = unit_file_path(project, version);
    let enable = manager
        .enable_unit_files(vec![unit_file_path.clone()], false, true)
        .await;

    match enable {
        Ok(unit) => {
            info!("Enabled unit: {:?}", unit);
        }
        Err(zbus::Error::MethodError(name, _, _))
            if name == "org.freedesktop.systemd1.NoSuchUnit" =>
        {
            warn!("No such unit file: {}", unit_file_path);
        }
        Err(e) => {
            error!("Failed to enable unit file: {}, {}", unit_file_path, e);
        }
    }
}

/// A route to enable a systemd service. This is _fast_, and can be used to switch between versions quickly.
///
/// It requires that a service be _installed_ first on the host, done with the "POST /service-installation" endpoint.
async fn enable_service(Json(req): Json<EnableServiceRequest>) -> impl IntoResponse {
    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .expect("manager init");

    let unit_name = unit_name(&req.project);

    enable_unit(&manager, &req.project, &req.version).await;

    manager.reload().await.expect("reload");

    info!("Starting service...");
    manager
        .reload_or_restart_unit(unit_name.clone(), "replace".to_string())
        .await
        .expect("systemd reload");

    let mut queued = true;
    while queued {
        info!("Polling for job to start...");
        queued = manager
            .list_jobs()
            .await
            .unwrap()
            .iter()
            .any(|(_, some_unit_name, _, _, _, _)| *some_unit_name == unit_name);

        sleep(Duration::from_millis(250)).await;
    }

    let units = manager
        .list_units_by_names(vec![unit_name.clone()])
        .await
        .unwrap();
    let unit = units.first().unwrap();
    let (_, _, loaded_state, active_state, _, _, _, _, _, _) = unit;

    match (loaded_state.as_str(), active_state.as_str()) {
        ("loaded", "active") => {
            info!("Service running!");
        }
        (loaded_state, active_state) => {
            panic!("Unrecognized state, loaded: {loaded_state}, active: {active_state}");
        }
    }

    (StatusCode::OK, "Service enabled.")
}

#[derive(Serialize, Deserialize)]
pub struct InstallServiceRequest {
    /// The name of the project, e.g. "ant-data-farm".
    pub project: String,

    /// The unique version ID of the software, corresponds to a path on the host. Reinstalling the same
    /// version multiple times is still fine, but there may be files in the 'cwd' that the process doesn't
    /// expect.
    pub version: String,

    /// If the service is a docker project, we require loading the image. Assumes the image is contained
    /// in the deployment TAR file is called "docker-image.tar" and will install that tagged image.
    pub is_docker: Option<bool>,

    /// The list of secret IDs/names that this project needs, e.g. ["jwt", "ant_fs_client_creds", ...]
    /// These secrets are replicated into the right directory for ant_library::load_secret() to find them
    /// when they are needed.
    pub secrets: Option<Vec<String>>,
}

fn deployment_file_name(project: &str, version: &str) -> String {
    format!("deployment.{project}.{version}.tar.gz")
}

/// The directory where all installable files for the project will live
fn install_location_path(install_root: &PathBuf, project: &str, version: &str) -> PathBuf {
    install_root.join(project).join(version)
}

/// The directory where the secrets files will be replicated
fn secrets_dir(install_dir: &PathBuf) -> PathBuf {
    install_dir.join("secrets")
}

async fn install_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<InstallServiceRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let file_name = deployment_file_name(&req.project, &req.version);
    let file_path = state.archive_root_dir.join(file_name);
    info!(
        "Installing {} version {} from {}...",
        req.project,
        req.version,
        file_path.display()
    );

    let file = std::fs::File::open(&file_path).map_err(|e| match e.kind() {
        ErrorKind::NotFound => {
            error!("No such deployment file {}: {e}", &file_path.display());
            return (
                StatusCode::BAD_REQUEST,
                format!(
                    "No deployment tarball found for: {} version {}",
                    req.project, req.version
                ),
            );
        }
        _ => {
            error!("Unknown error {}: {e}", file_path.display());
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid deployment file: {}", &file_path.display()),
            );
        }
    })?;

    let file = flate2::read::GzDecoder::new(file);

    let mut deployment = tar::Archive::new(file);

    let dst = install_location_path(&state.install_root_dir, &req.project, &req.version);
    std::fs::create_dir_all(dst.parent().unwrap()).expect("mkdir");

    info!("Unpacking installation to: {}", &dst.display());
    deployment.unpack(&dst).expect("unpack");

    let num_secrets = req.secrets.as_ref().map(|s| s.len()).unwrap_or(0);
    info!("Finding {} secret(s)...", num_secrets);
    let dst_secrets_dir = secrets_dir(&dst);
    std::fs::create_dir_all(&dst_secrets_dir).expect("mkdir secrets");

    for secret in req.secrets.unwrap_or(vec![]) {
        info!("Copying secret {secret}...");
        let source_secret =
            ant_library::secret::find_secret(&secret, Some(state.secrets_root_dir.clone()));

        let dst_secret = dst_secrets_dir.join(ant_library::secret::secret_name(&secret));
        std::fs::copy(source_secret, dst_secret).map_err(|e| match e.kind() {
            ErrorKind::NotFound => {
                error!("Failed to find secret {secret}: {e}");
                return (StatusCode::BAD_REQUEST, format!("Invalid secret: {secret}"));
            }
            _ => {
                error!("Unknown error occurred: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error, please retry.".to_string(),
                );
            }
        })?;
    }

    if req.is_docker.unwrap_or(false) {
        info!("Loading docker image...");
        let docker_img_path = dst.join("docker-image.tar");
        if !std::fs::exists(&docker_img_path).unwrap() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Docker mentioned bu no docker-image.tar included".to_string(),
            ));
        }

        let docker_img_file = File::open(docker_img_path).await.unwrap();
        let docker_img_bytes = codec::FramedRead::new(docker_img_file, codec::BytesCodec::new())
            .map(|r| r.unwrap().freeze());

        let docker =
            bollard::Docker::connect_with_defaults().expect("docker daemon connect failed");

        let mut import_out = docker.import_image_stream(
            bollard::query_parameters::ImportImageOptions {
                ..Default::default()
            },
            docker_img_bytes,
            None,
        );

        while let Some(val) = import_out.next().await {
            let build_info = val.expect("docker connection");
            if build_info.error.is_some() || build_info.error_detail.is_some() {
                error!(
                    "Failed to load image: {}",
                    build_info.error.unwrap_or("".to_string())
                );
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Failed to load docker image.".to_string(),
                ));
            }

            if let Some(stream) = &build_info.stream {
                info!("load docker image: {}", stream.trim());

                if stream.contains("Loaded image") {
                    break;
                }
            }

            warn!("Unknown value: {:#?}", build_info);
        }
    }

    Ok((StatusCode::OK, "Service installed."))
}

async fn disable_unit(manager: &ManagerProxy<'_>, unit_name: &String) {
    info!("Disabling service {unit_name}...");
    let disable = manager
        .disable_unit_files(vec![unit_name.clone()], false)
        .await;
    match disable {
        Ok(job) => {
            info!("Service disabled: {job:?}");
        }
        Err(zbus::Error::MethodError(name, msg, _)) => {
            info!("No such service running: {name}, {msg:?}. Ignoring...");
        }
        Err(e) => {
            panic!("Disabling previous unit failed: {e}");
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct DisableServiceRequest {
    project: String,
}

async fn disable_service(Json(req): Json<DisableServiceRequest>) -> impl IntoResponse {
    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .expect("manager init");

    let unit_name = unit_name(&req.project);

    disable_unit(&manager, &unit_name).await;
    manager
        .kill_unit(unit_name, "all".to_string(), 9)
        .await
        .unwrap();

    manager.reload().await.expect("reload");

    (StatusCode::OK, "Service disabled.")
}

pub fn make_routes() -> Router<AntHostAgentState> {
    Router::new()
        .route_with_tsr("/service", post(enable_service).delete(disable_service))
        .route_with_tsr("/service-installation", post(install_service))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /service/service",
                "DELETE /service/service",
                "POST /service/service-installation",
            ])
        })
}
