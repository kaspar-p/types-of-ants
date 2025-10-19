use std::time::Duration;

use axum::{response::IntoResponse, routing::post, Json, Router};
use axum_extra::routing::RouterExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, warn};
use zbus_systemd::{systemd1::ManagerProxy, zbus};

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

pub fn make_routes() -> Router {
    Router::new().route_service_with_tsr("/service", post(enable_service).delete(disable_service))
}
