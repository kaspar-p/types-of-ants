use std::time::Duration;

use axum::{response::IntoResponse, routing::post, Json, Router};
use http::{header, Method, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, info};
use zbus_systemd::zbus;

use crate::state::AntZookeeperState;

pub mod state;

#[derive(Serialize, Deserialize)]
pub struct EnableServiceRequest {
    pub project: String,
    pub version: String,
}

pub async fn enable_service(Json(req): Json<EnableServiceRequest>) -> impl IntoResponse {
    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .expect("manager init");

    let unit_name = format!("{}.service", req.project);

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

    info!("Enabling service...");
    let unit_file_path = format!(
        "/home/ant/service/{}/{}/{}",
        req.project, req.version, unit_name
    );
    manager
        .enable_unit_files(vec![unit_file_path.clone()], false, true)
        .await
        .expect("systemd enable");

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

pub fn make_routes(s: AntZookeeperState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Router::new()
        .route("/enable-service", post(enable_service))
        .with_state(s)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(CatchPanicLayer::custom(ant_library::middleware_catch_panic))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware_print_request_response,
                ))),
        );

    return Ok(app);
}
