use std::{path::PathBuf, time::Duration};

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, warn};
use zbus_systemd::{systemd1::ManagerProxy, zbus};

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
        queued = manager.list_jobs().await.unwrap().iter().any(
            |(queue_id, some_unit_name, job_type, job_state, job_obj_path, unit_obj_path)| {
                *some_unit_name == unit_name
            },
        );

        sleep(Duration::from_millis(25)).await;
    }

    let units = manager
        .list_units_by_names(vec![unit_name.clone()])
        .await
        .unwrap();
    let unit = units.first().unwrap();
    let (
        unit_name,
        desc,
        loaded_state,
        active_state,
        substate,
        _,
        unit_obj_path,
        job_queue_id,
        job_type,
        job_obj_path,
    ) = unit;

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

    ///
    pub version: String,

    /// If the service is a docker project, we require loading the image. Assumes the image is contained
    /// in the deployment TAR file is called "docker-image.tar" and will install that tagged image.
    pub is_docker: bool,
}

fn deployment_file_name(project: &str, version: &str) -> String {
    format!("deployment.{project}.{version}.tar.gz")
}

fn install_location_path(install_root: &PathBuf, project: &str, version: &str) -> PathBuf {
    install_root.join(project).join(version)
}

async fn install_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<InstallServiceRequest>,
) -> impl IntoResponse {
    let file_name = deployment_file_name(&req.project, &req.version);
    let file_path = state.archive_root_dir.join(file_name);
    info!(
        "Installing {} version {} from {}...",
        req.project,
        req.version,
        file_path.display()
    );

    let file = std::fs::File::open(&file_path)
        .expect(&format!("valid deployment file {}", &file_path.display()));

    let file = flate2::read::GzDecoder::new(file);

    let mut deployment = tar::Archive::new(file);

    deployment
        .unpack(install_location_path(
            &state.install_root_dir,
            &req.project,
            &req.version,
        ))
        .expect("unpack");

    (StatusCode::OK, "Service installed.")
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
}
