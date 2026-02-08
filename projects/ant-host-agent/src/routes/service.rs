use ant_library::headers::{XAntProjectHeader, XAntVersionHeader};
use flate2::read::GzDecoder;
use humansize::DECIMAL;
use std::{io::ErrorKind, path::PathBuf, time::Duration};
use tokio_util::codec;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use axum_extra::{routing::RouterExt, TypedHeader};
use futures_util::stream::StreamExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt, time::sleep};
use tracing::{debug, error, info, warn};
use zbus_systemd::{systemd1::ManagerProxy, zbus};

use std::default::Default;

use crate::{err::AntHostAgentError, state::AntHostAgentState};

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

        sleep(Duration::from_millis(500)).await;
    }

    let mut activating = true;
    while activating {
        let units = manager
            .list_units_by_names(vec![unit_name.clone()])
            .await
            .unwrap();
        let unit = units.first().unwrap();
        let (_, _, loaded_state, active_state, _, _, _, _, _, _) = unit;

        info!("Polling for job to activate: {unit:?}");
        activating = match (loaded_state.as_str(), active_state.as_str()) {
            ("loaded", "activating") => true,
            ("loaded", "active") => false,
            (_, "failed") => {
                return (StatusCode::UNPROCESSABLE_ENTITY, "Service failed to start.");
            }
            (loaded_state, active_state) => {
                panic!("Unrecognized state, loaded: {loaded_state}, active: {active_state}");
            }
        };

        sleep(Duration::from_millis(500)).await;
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
}

fn deployment_file_name(project: &str, version: &str) -> String {
    format!("deployment.{project}.{version}.tar.gz")
}

/// The directory where all installable files for the project will live
fn install_location_path(install_root: &PathBuf, project: &str, version: &str) -> PathBuf {
    install_root.join(project).join(version)
}

async fn install_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<InstallServiceRequest>,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let file_name = deployment_file_name(&req.project, &req.version);
    let file_path = state.archive_root_dir.join(file_name);
    info!(
        "Installing {} version {} from {}...",
        req.project,
        req.version,
        file_path.display()
    );

    let file = std::fs::File::open(&file_path).map_err(|e| match e.kind() {
        ErrorKind::NotFound => AntHostAgentError::validation_msg(
            format!(
                "No deployment tarball found for: {} version {}",
                req.project, req.version
            )
            .as_str(),
        ),
        _ => AntHostAgentError::InternalServerError(Some(e.into())),
    })?;

    let mut deployment = tar::Archive::new(GzDecoder::new(file));

    let dst = install_location_path(&state.install_root_dir, &req.project, &req.version);
    std::fs::create_dir_all(&dst)?;

    info!("Unpacking installation to: {}", dst.display());
    deployment.unpack(&dst)?;

    let template_variables = mustache::MapBuilder::new()
        .insert_str("INSTALL_DIR", dst.to_str().unwrap())
        .build();

    let unit_file_path = dst.join(unit_name(&req.project));
    let unit_file_template = mustache::compile_path(&unit_file_path)?;

    info!("Rewriting unit file with data: {:?}", template_variables);
    unit_file_template.render_data(
        &mut std::fs::File::create(&unit_file_path)?,
        &template_variables,
    )?;

    let docker_img_path = dst.join("docker-image.tar");
    if std::fs::exists(&docker_img_path)? {
        info!("Loading docker image...");

        let docker_img_file = File::open(docker_img_path).await.unwrap();
        let docker_img_bytes = codec::FramedRead::new(docker_img_file, codec::BytesCodec::new())
            .map(|r| r.unwrap().freeze());

        debug!("Connecting to docker daemon...");
        let docker =
            bollard::Docker::connect_with_defaults().expect("docker daemon connect failed");

        let mut import_out = docker.import_image_stream(
            bollard::query_parameters::ImportImageOptions {
                ..Default::default()
            },
            docker_img_bytes,
            None,
        );

        debug!("Start loading docker image...");
        while let Some(val) = import_out.next().await {
            let build_info = val.expect("docker connection");
            if build_info.error.is_some() || build_info.error_detail.is_some() {
                error!(
                    "Failed to load image: {}",
                    build_info.error.unwrap_or("".to_string())
                );
                return Err(AntHostAgentError::validation_msg(
                    "Failed to load docker image.",
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

async fn register_service(
    State(state): State<AntHostAgentState>,
    TypedHeader(project): TypedHeader<XAntProjectHeader>,
    TypedHeader(version): TypedHeader<XAntVersionHeader>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let path = state
        .archive_root_dir
        .join(deployment_file_name(&project.0, &version.0));
    let mut file = File::create(&path).await?;

    let mut field = multipart
        .next_field()
        .await
        .map_err(|e| {
            AntHostAgentError::validation("No field found in multipart request!", Some(e.into()))
        })?
        .ok_or(AntHostAgentError::validation_msg(
            "No bytes field found in request!",
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

    Ok((StatusCode::OK, "Service registered."))
}

pub fn make_routes() -> Router<AntHostAgentState> {
    Router::new()
        .route_with_tsr("/service", post(enable_service).delete(disable_service))
        .route_with_tsr("/service-installation", post(install_service))
        .route_with_tsr(
            "/service-registration",
            post(register_service).layer(
                DefaultBodyLimit::max(1000 * 1000 * 1000), // 1GB
            ),
        )
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /service/service",
                "DELETE /service/service",
                "POST /service/service-installation",
                "POST /service/service-registration",
            ])
        })
}
