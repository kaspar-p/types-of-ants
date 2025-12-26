use std::path::PathBuf;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::post,
    Router,
};
use axum_extra::{headers::Header, routing::RouterExt, TypedHeader};
use http::{HeaderName, HeaderValue, StatusCode};
use humansize::DECIMAL;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};
use tracing::info;

use crate::{err::AntZookeeperError, state::AntZookeeperState};

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

#[derive(Serialize, Deserialize)]
pub struct CreateServiceVersionRequest {
    project: String,
    version: String,
}

static X_ANT_PROJECT_HEADER: HeaderName = http::HeaderName::from_static("x-ant-project");
struct XAntProjectHeader(pub String);

impl Header for XAntProjectHeader {
    fn name() -> &'static http::HeaderName {
        &X_ANT_PROJECT_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i http::HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum_extra::headers::Error::invalid)?;

        let value = value
            .to_str()
            .map_err(|_| axum_extra::headers::Error::invalid())?
            .to_string();

        Ok(Self(value))
    }

    fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.0).expect("invalid header value stored");
        values.extend(std::iter::once(value));
    }
}

static X_ANT_VERSION_HEADER: HeaderName = http::HeaderName::from_static("x-ant-version");
struct XAntVersionHeader(pub String);

impl Header for XAntVersionHeader {
    fn name() -> &'static http::HeaderName {
        &X_ANT_VERSION_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i http::HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum_extra::headers::Error::invalid)?;

        let value = value
            .to_str()
            .map_err(|_| axum_extra::headers::Error::invalid())?
            .to_string();

        Ok(Self(value))
    }

    fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.0).expect("invalid header value stored");
        values.extend(std::iter::once(value));
    }
}

fn persist_dir(root_dir: &PathBuf) -> PathBuf {
    root_dir.join("services-db")
}

fn service_file_name(project: &str, version: &str) -> String {
    format!("{project}.{version}.bld")
}

/// An API to ingest a new version of a service, generally a new binary for that version/
async fn register_service_version(
    State(state): State<AntZookeeperState>,
    TypedHeader(project): TypedHeader<XAntProjectHeader>,
    TypedHeader(version): TypedHeader<XAntVersionHeader>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.get_project(&project.0).await? {
        info!("Registering project [{}] for the first time...", project.0);
        // Building our own images means an owned project!
        state.db.register_project(&project.0, true).await?;
    }

    let dir = persist_dir(&state.root_dir);
    create_dir_all(&dir).await?;

    let path = dir.join(service_file_name(&project.0, &version.0));
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

    state
        .db
        .register_project_version(&project.0, &version.0)
        .await?;

    Ok((StatusCode::OK, "Version registered"))
}

pub fn make_routes() -> Router<AntZookeeperState> {
    Router::new().route_with_tsr(
        "/service-version",
        post(register_service_version).layer(
            DefaultBodyLimit::max(1000 * 1000 * 1000), // 1GB
        ),
    )
}
