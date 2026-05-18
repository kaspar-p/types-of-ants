use ant_library::{
    anthill::{AnthillArchetype, AnthillManifest, AnthillManifestError},
    headers::{XAntProjectHeader, XAntServiceIdHeader, XAntVersionHeader},
};
use anyhow::Context;
use flate2::read::GzDecoder;
use handlebars::Handlebars;
use humansize::DECIMAL;
use std::{collections::HashMap, io::ErrorKind, path::PathBuf, str::FromStr};
use tempfile::TempDir;
use tokio_util::codec;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::{routing::RouterExt, TypedHeader};
use futures_util::stream::StreamExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, error, info, warn};
use zbus_systemd::{systemd1::ManagerProxy, zbus};

use std::default::Default;

use crate::{
    err::AntHostAgentError,
    state::{AntHostAgentState, HostService},
    systemd::{restart_unit, SystemdUnitError},
};

fn unit_name(service_id: &str) -> String {
    format!("{service_id}.service")
}

fn unit_file_path(service_id: &str, version: &str) -> PathBuf {
    PathBuf::from("/")
        .join("home")
        .join("ant")
        .join("service")
        .join(service_id)
        .join(version)
        .join(unit_name(service_id))
}

#[derive(Serialize, Deserialize)]
pub struct EnableServiceRequest {
    #[deprecated(note = "Prefer service_id")]
    pub project: Option<String>,
    pub service_id: Option<String>,

    pub version: String,
}

async fn enable_unit(manager: &ManagerProxy<'_>, service_id: &str, version: &str) {
    info!("Enabling service...");
    let unit_file_path = unit_file_path(service_id, version);
    let enable = manager
        .enable_unit_files(
            vec![unit_file_path.to_str().unwrap().to_string()],
            false,
            true,
        )
        .await;

    match enable {
        Ok(unit) => {
            info!("Enabled unit: {:?}", unit);
        }
        Err(zbus::Error::MethodError(name, _, _))
            if name == "org.freedesktop.systemd1.NoSuchUnit" =>
        {
            warn!("No such unit file: {}", unit_file_path.display());
        }
        Err(e) => {
            error!(
                "Failed to enable unit file: {}, {}",
                unit_file_path.display(),
                e
            );
        }
    }
}

/// A route to enable a systemd service. This is _fast_, and can be used to switch between versions quickly.
///
/// It requires that a service be _installed_ first on the host, done with the "POST /service-installation" endpoint.
async fn enable_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<EnableServiceRequest>,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .context("manager init")?;

    let service_id = compute_service_id(req.project.as_deref(), req.service_id.as_deref())?;

    let unit_name = unit_name(&service_id);

    enable_unit(&manager, &service_id, &req.version).await;

    manager.reload().await.context("daemon reload")?;

    info!("Starting service...");

    match restart_unit(&manager, &unit_name).await? {
        Ok(_) => {}
        Err(SystemdUnitError::UnitTookTooLongToStart)
        | Err(SystemdUnitError::UnitFailedToStart) => {
            return Ok((StatusCode::UNPROCESSABLE_ENTITY, "Service failed to start."));
        }
        Err(SystemdUnitError::UnrecognizedState(loaded, active)) => {
            return Err(AntHostAgentError::InternalServerError(Some(
                anyhow::Error::msg(format!(
                    "Unrecognized state, loaded: {loaded}, active: {active}"
                )),
            )));
        }
    }

    let install_dir = install_location_path(&state.install_root_dir, service_id, &req.version);
    let manifest = AnthillManifest::from_file(&install_dir.join("anthill.json"))?;

    state
        .services
        .lock()
        .await
        .insert(service_id.to_string(), HostService { manifest });

    Ok((StatusCode::OK, "Service enabled."))
}

#[derive(Serialize, Deserialize)]
pub struct InstallServiceRequest {
    /// The name of the project, e.g. "ant-data-farm".
    #[deprecated(note = "Prefer service_id")]
    pub project: Option<String>,
    pub service_id: Option<String>,

    /// The unique version ID of the software, corresponds to a path on the host. Reinstalling the same
    /// version multiple times is still fine, but there may be files in the 'cwd' that the process doesn't
    /// expect.
    pub version: String,
}

fn deployment_file_name(service_id: &str, version: &str) -> String {
    format!("deployment.{service_id}.{version}.tar.gz")
}

/// The directory where all installable files for the project will live
fn install_location_path(install_root: &PathBuf, service_id: &str, version: &str) -> PathBuf {
    install_root.join(service_id).join(version)
}

fn temp_dir(state: &AntHostAgentState) -> Result<TempDir, anyhow::Error> {
    let global_temp_dir = state.archive_root_dir.join("tmp");
    std::fs::create_dir_all(&global_temp_dir)?;

    Ok(tempfile::tempdir_in(global_temp_dir)?)
}

async fn install_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<InstallServiceRequest>,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let service_id = compute_service_id(req.project.as_deref(), req.service_id.as_deref())?;

    let file_name = deployment_file_name(&service_id, &req.version);
    let file_path = state.archive_root_dir.join(file_name);
    info!(
        "Installing [{}] version [{}] from [{}]...",
        service_id,
        req.version,
        file_path.display()
    );

    let file = std::fs::File::open(&file_path).map_err(|e| match e.kind() {
        ErrorKind::NotFound => AntHostAgentError::validation_msg(
            format!(
                "No deployment tarball found for: {} version {}",
                service_id, req.version
            )
            .as_str(),
        ),
        _ => AntHostAgentError::InternalServerError(Some(e.into())),
    })?;

    let dst = {
        let tmp_dst = temp_dir(&state)?;

        let mut deployment = tar::Archive::new(GzDecoder::new(file));
        info!("Unpacking installation to: {}", tmp_dst.path().display());
        deployment
            .unpack(&tmp_dst)
            .context("attempted to unpack malformed tarfile")?;

        let manifest = match AnthillManifest::from_file(&tmp_dst.path().join("anthill.json")) {
            Ok(manifest) => Ok(Some(manifest)),
            Err(AnthillManifestError::Io(e)) if matches!(e.kind(), ErrorKind::NotFound) => Ok(None),
            Err(e) => Err(e),
        }?;

        let versioned = manifest
            .and_then(|m| m.deployment)
            .and_then(|d| d.versioned)
            .unwrap_or(true);

        let version = if versioned { &req.version } else { "service" };
        let dst = install_location_path(&state.install_root_dir, &service_id, &version);

        info!("Copying installation to: {}", dst.display());
        std::fs::create_dir_all(&dst)?;
        dircpy::copy_dir_advanced(&tmp_dst, &dst, true, true, true, vec![], vec![])
            .context("move deployment to final location")?;

        dst
    };

    let unit_file_path = dst.join(unit_name(&service_id));
    if std::fs::exists(&unit_file_path)? {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        handlebars
            .register_template_file("systemd", &unit_file_path)
            .context("handlebars compilation")?;

        let mut variables = ant_library::env::env_vars_to_map(&dst.join(".env"))?;

        variables.insert(
            "INSTALL_DIR".to_string(),
            dst.to_str()
                .expect("destination was not string")
                .to_string(),
        );

        info!("Rendering systemd template: {}", unit_file_path.display());
        let content = handlebars
            .render("systemd", &variables)
            .map_err(|e| match e.reason() {
                handlebars::RenderErrorReason::MissingVariable(Some(var)) => {
                    debug!("Replacement variables: {:#?}", variables);
                    return AntHostAgentError::validation_msg(&format!(
                        "Unknown template variable replacement attempt of [{var}] in file: {}",
                        unit_name(&service_id)
                    ));
                }
                r => {
                    error!("Failed to render template: {r}");
                    return AntHostAgentError::validation_msg(&format!(
                        "Failed to replace template: {}",
                        unit_name(&service_id)
                    ));
                }
            })?;

        info!("Rewriting unit file with data...");
        std::fs::write(unit_file_path, content)?;
    }

    let docker_img_path = dst.join("docker-image.tar");
    if std::fs::exists(&docker_img_path).context("docker img exists failed")? {
        info!("Loading docker image...");

        let docker_img_file = File::open(docker_img_path)
            .await
            .expect("opening docker img failed");
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
    #[deprecated(note = "Prefer service_id")]
    project: Option<String>,
    /// backwards compat optional.
    service_id: Option<String>,
}

async fn disable_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<DisableServiceRequest>,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let service_id = compute_service_id(req.project.as_deref(), req.service_id.as_deref())?;

    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .expect("manager init");

    let unit_name = unit_name(&service_id);

    disable_unit(&manager, &unit_name).await;
    manager
        .kill_unit(unit_name, "all".to_string(), 9)
        .await
        .unwrap();

    manager.reload().await.expect("reload");

    // Delete that from in-memory db of services
    state.services.lock().await.remove(service_id);

    Ok((StatusCode::OK, "Service disabled."))
}

fn compute_service_id<'a>(
    project_header: Option<&'a str>,
    service_id_header: Option<&'a str>,
) -> Result<&'a str, AntHostAgentError> {
    let service_id = service_id_header
        .as_ref()
        .or(project_header.as_ref())
        .ok_or(AntHostAgentError::validation_msg(
            "One of X-Ant-Project or X-Ant-Service-Id must be specified",
        ))?;

    Ok(service_id)
}

async fn register_service(
    State(state): State<AntHostAgentState>,
    TypedHeader(project): TypedHeader<XAntProjectHeader>,
    TypedHeader(service_id): TypedHeader<XAntServiceIdHeader>,
    TypedHeader(version): TypedHeader<XAntVersionHeader>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let service_id = compute_service_id(project.0.as_deref(), service_id.0.as_deref())?;

    let path = state
        .archive_root_dir
        .join(deployment_file_name(&service_id, &version.0));
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceDiscoveryResponse(pub Vec<ServiceDiscoveryTargets>);

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceDiscoveryTargets {
    /// List of URLs, e.g. "localhost:1234"
    targets: Vec<String>,
    labels: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServiceDiscoveryFilters {
    pub archetype: Option<String>,
}

async fn get_service_discovery(
    State(state): State<AntHostAgentState>,
    Query(filters): Query<ServiceDiscoveryFilters>,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let mut targets = vec![];
    for (service_id, service) in state.services.lock().await.iter() {
        if let Some(archetype) = &filters.archetype {
            let archetype = AnthillArchetype::from_str(&archetype)
                .map_err(|e| AntHostAgentError::validation_msg(&e))?;

            if let Some(service_archetype) = &service.manifest.archetype {
                if archetype != *service_archetype {
                    warn!("Skipping {service_id} because archetype mismatch.");
                    continue;
                }
            }
        }

        match service.manifest.deployment.as_ref().and_then(|d| d.port) {
            None => {
                warn!("Skipping {service_id} because no port is defined in the anthill.json manifest!");
                continue;
            }
            Some(port) => {
                targets.push(ServiceDiscoveryTargets {
                    targets: vec![format!("localhost:{}", port)],
                    labels: HashMap::from([
                        ("project".to_string(), service.manifest.project.to_string()),
                        ("service_id".to_string(), service_id.to_string()),
                    ]),
                });
            }
        }
    }

    Ok((StatusCode::OK, Json(targets)))
}

pub fn make_routes() -> Router<AntHostAgentState> {
    Router::new()
        .route_with_tsr("/sd", get(get_service_discovery))
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
