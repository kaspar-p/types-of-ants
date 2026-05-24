use ant_library::{
    anthill::{AnthillArchetype, AnthillManifest, AnthillManifestError},
    headers::{XAntServiceIdHeader, XAntVersionHeader},
};
use anyhow::Context;
use flate2::read::GzDecoder;
use handlebars::{no_escape, Handlebars};
use humansize::DECIMAL;
use std::{
    collections::HashMap,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
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
use tracing::{debug, error, info, instrument, warn};
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

#[derive(Serialize, Deserialize)]
pub struct EnableServiceRequest {
    pub service_id: String,

    pub version: String,
}

#[instrument(skip(manager))]
async fn enable_unit(manager: &ManagerProxy<'_>, unit_path: &Path) -> Result<(), anyhow::Error> {
    info!("Enabling service...");
    let enable = manager
        .enable_unit_files(vec![unit_path.to_str().unwrap().to_string()], false, true)
        .await;

    match enable {
        Ok(unit) => {
            info!("Enabled unit: {:?}", unit);
        }
        Err(zbus::Error::MethodError(name, _, _))
            if name == "org.freedesktop.systemd1.NoSuchUnit" =>
        {
            warn!("No such unit file: {}", unit_path.display());
        }
        Err(e) => {
            error!("Failed to enable unit file: {}, {}", unit_path.display(), e);
            return Err(anyhow::Error::from(e));
        }
    }

    Ok(())
}

/// Make a symlink `symlink` pointing to `to`, atomically, via making a temp file and moving it into place.
fn symlink(symlink: &Path, to: &Path) -> Result<(), anyhow::Error> {
    info!(
        "Changing symlink {} to point to {}",
        symlink.display(),
        to.display()
    );
    let tmp_link = symlink.with_added_extension("new");

    info!("Creating temporary: {}", tmp_link.display());
    std::os::unix::fs::symlink(to, &tmp_link).context("creating <symlink>.new")?;

    info!("Renaming {} to {}", tmp_link.display(), symlink.display());
    std::fs::rename(&tmp_link, &symlink).context("renaming <symlink>.new to <symlink>")?;

    Ok(())
}

/// Every project has the same-looking systemd file, linked once on startup. It delegates
/// to the relevant version by resolving through symlinks in the ExecStart declaration.
fn systemd_unit_file_content(service_id: &str) -> String {
    format!(
        "[Unit]
Description={service_id}

[Service]
Type=simple
EnvironmentFile=/home/ant/service/{service_id}/current/.env
Environment=TYPESOFANTS_SECRET_DIR=/home/ant/service/{service_id}/current/secrets
ExecStart=/home/ant/service/{service_id}/current/run.sh
WorkingDirectory=/home/ant/service/{service_id}/current
Slice=typesofants.slice
Restart=always

[Install]
WantedBy=multi-user.target
"
    )
}

/// Creates a global systemd unit file for this service, the ExecStart of which points to
/// the symlinked "current" directory, which can switch per version.
#[instrument]
async fn ensure_systemd(service_id: &str) -> Result<(), anyhow::Error> {
    let unit_path = PathBuf::from("/")
        .join("etc")
        .join("systemd")
        .join("system")
        .join(unit_name(service_id));

    let content = systemd_unit_file_content(service_id);

    if !std::fs::exists(&unit_path)?
        || std::fs::read_to_string(&unit_path).context("read systemd unit")? != content
    {
        info!(
            "The systemd file {} didn't exist or was outdated",
            unit_path.display()
        );

        std::fs::write(&unit_path, content).context("write systemd")?;

        let conn = zbus::Connection::system().await.expect("system connection");
        let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
            .await
            .context("manager init")?;

        enable_unit(&manager, &unit_path).await?;

        manager.reload().await.context("daemon reload")?;
    } else {
        info!("The systemd file was healthy, skipping.");
    }

    return Ok(());
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

    let unit_name = unit_name(&req.service_id);

    let previous_dir = state
        .install_root_dir
        .join(&req.service_id)
        .join("previous");
    let current_dir = state.install_root_dir.join(&req.service_id).join("current");

    let active_version_dir: Option<PathBuf> = if std::fs::exists(&current_dir)? {
        let active_version_dir =
            std::fs::canonicalize(&current_dir).context("resolve current symlink")?;
        info!("current -> {}", active_version_dir.display());
        Some(active_version_dir)
    } else {
        None
    };

    let incoming_version_dir = resolve_latest_versioned_install_dir(
        &state.install_root_dir,
        &req.service_id,
        &req.version,
    )?
    .ok_or(AntHostAgentError::validation_msg(
        "No installation directories can be found, did you install first?",
    ))?;

    if let Some(active_version_dir) = active_version_dir {
        info!("Symlinking new previous");
        symlink(&previous_dir, &active_version_dir).context("symlink previous")?;
    }
    info!("Symlinking new current...");
    symlink(&current_dir, &incoming_version_dir).context("symlink current")?;

    ensure_systemd(&req.service_id)
        .await
        .context("ensure systemd")?;

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

    let manifest = AnthillManifest::from_file(&current_dir.join("anthill.json"))?;

    state
        .services
        .lock()
        .await
        .insert(req.service_id.to_string(), HostService { manifest });

    Ok((StatusCode::OK, "Service enabled."))
}

#[derive(Serialize, Deserialize)]
pub struct InstallServiceRequest {
    pub service_id: String,

    /// The unique version ID of the software, corresponds to a path on the host. Reinstalling the same
    /// version multiple times is still fine, but there may be files in the 'cwd' that the process doesn't
    /// expect.
    pub version: String,
}

fn deployment_file_name(service_id: &str, version: &str) -> String {
    format!("deployment.{service_id}.{version}.tar.gz")
}

fn versioned_install_dir(
    install_root: &PathBuf,
    service_id: &str,
    version: &str,
) -> Result<PathBuf, anyhow::Error> {
    let base_dir = install_root.join(service_id);

    let mut attempt = 1;

    loop {
        let candidate = base_dir.join(format!("{version}.{attempt}"));
        if !std::fs::exists(&candidate)? {
            return Ok(candidate);
        } else {
            attempt += 1
        }
    }
}

fn resolve_latest_versioned_install_dir(
    install_root: &PathBuf,
    service_id: &str,
    version: &str,
) -> Result<Option<PathBuf>, anyhow::Error> {
    let base_dir = install_root.join(service_id);
    if !std::fs::exists(&base_dir)? {
        return Ok(None);
    }

    let mut latest: Option<(i32, PathBuf)> = None;

    for entry in std::fs::read_dir(&base_dir)? {
        let entry = entry?;
        let dir_name = entry.file_name().into_string().expect("file was not utf8");

        // From v523-2025-12-12-01-01-01-deadbeef.99, extract the .99
        if let Some(dir_attempt) = dir_name.strip_prefix(&format!("{version}.")) {
            let dir_attempt = dir_attempt.parse::<i32>().expect(&format!(
                "entry {dir_name} in {} malformed attempt number!",
                base_dir.display()
            ));

            // If 99 is the highest we've seen so far, set best.
            if latest
                .as_ref()
                .map(|(best_attempt, _)| dir_attempt > *best_attempt)
                .unwrap_or(true)
            {
                latest = Some((dir_attempt, entry.path()));
            }
        }
    }

    Ok(latest.map(|l| l.1))
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
    let file_name = deployment_file_name(&req.service_id, &req.version);
    let file_path = state.archive_root_dir.join(file_name);
    info!(
        "Installing [{}] version [{}] from [{}]...",
        req.service_id,
        req.version,
        file_path.display()
    );

    let file = std::fs::File::open(&file_path).map_err(|e| match e.kind() {
        ErrorKind::NotFound => AntHostAgentError::validation_msg(
            format!(
                "No deployment tarball found for: {} version {}",
                req.service_id, req.version
            )
            .as_str(),
        ),
        _ => AntHostAgentError::InternalServerError(Some(e.into())),
    })?;

    let versioned_install_dir = {
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
        let dst = versioned_install_dir(&state.install_root_dir, &req.service_id, &version)?;

        info!(
            "Copying installation from [{}] to [{}]",
            tmp_dst.path().display(),
            dst.display()
        );
        dircpy::CopyBuilder::new(&tmp_dst, &dst)
            .overwrite(true)
            .with_progress(|total, cnt| info!("Replaced file: {cnt}/{total}"))
            .run()
            .context("copy deployment files to final location")?;

        dst
    };

    let systemd_unit_path = versioned_install_dir.join(unit_name(&req.service_id));
    if std::fs::exists(&systemd_unit_path)? {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(no_escape);

        handlebars
            .register_template_file("systemd", &systemd_unit_path)
            .context("handlebars compilation")?;

        let mut variables = ant_library::env::env_vars_to_map(&versioned_install_dir.join(".env"))?;

        variables.insert(
            "INSTALL_DIR".to_string(),
            versioned_install_dir
                .to_str()
                .expect("destination was not string")
                .to_string(),
        );

        info!(
            "Rendering systemd template: {}",
            systemd_unit_path.display()
        );
        let content = handlebars
            .render("systemd", &variables)
            .map_err(|e| match e.reason() {
                handlebars::RenderErrorReason::MissingVariable(Some(var)) => {
                    debug!("Replacement variables: {:#?}", variables);
                    return AntHostAgentError::validation_msg(&format!(
                        "Unknown template variable replacement attempt of [{var}] in file: {}",
                        unit_name(&req.service_id)
                    ));
                }
                r => {
                    error!("Failed to render template: {r}");
                    return AntHostAgentError::validation_msg(&format!(
                        "Failed to replace template: {}",
                        unit_name(&req.service_id)
                    ));
                }
            })?;

        let mut temp_unit_file =
            tempfile::NamedTempFile::new().context("create temp systemd unit file")?;
        info!(
            "Writing temp unit [{}] file with rendered data...",
            temp_unit_file.path().display()
        );
        temp_unit_file
            .write_all(content.as_bytes())
            .context("writing to temp systemd unit file")?;

        info!(
            "Replacing [{}] with [{}]",
            temp_unit_file.path().display(),
            systemd_unit_path.display()
        );
        std::fs::copy(temp_unit_file.path(), systemd_unit_path)
            .context("copying to systemd unit file")?;
    }

    let docker_img_path = versioned_install_dir.join("docker-image.tar");
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
    service_id: String,
}

async fn disable_service(
    State(state): State<AntHostAgentState>,
    Json(req): Json<DisableServiceRequest>,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .expect("manager init");

    let unit_name = unit_name(&req.service_id);

    disable_unit(&manager, &unit_name).await;
    manager
        .kill_unit(unit_name, "all".to_string(), 9)
        .await
        .unwrap();

    manager.reload().await.expect("reload");

    // Delete that from in-memory db of services
    state.services.lock().await.remove(&req.service_id);

    Ok((StatusCode::OK, "Service disabled."))
}

async fn register_service(
    State(state): State<AntHostAgentState>,
    TypedHeader(service_id): TypedHeader<XAntServiceIdHeader>,
    TypedHeader(version): TypedHeader<XAntVersionHeader>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AntHostAgentError> {
    let path = state
        .archive_root_dir
        .join(deployment_file_name(&service_id.0, &version.0));
    info!("Registering service: {}", path.display());
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use handlebars::{no_escape, Handlebars};

    #[test]
    fn handlebars_no_replace_equals() {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(no_escape);

        let mut variables = HashMap::new();
        variables.insert("VARIABLE".to_string(), "a=b");

        let output = handlebars
            .render_template("data {{VARIABLE}}", &variables)
            .unwrap();

        assert_eq!(output, "data a=b");
    }
}
