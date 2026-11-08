use std::format;

use ant_library::routes::Routes;
use anyhow::Context;
use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use bytes::Bytes;
use chrono::{DateTime, Datelike, Timelike, Utc};
use http::{header, Method, StatusCode};
use postgresql_commands::{pg_dump::PgDumpBuilder, traits::CommandToString, CommandBuilder};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};
use tracing::{debug, error, info, warn};

use crate::{
    rotation::{backup_key_prefix, RotationError},
    state::AntBackingItUpState,
    storage_client::DatabaseParams,
};

pub mod crypto;
mod rotation;
mod slots;
pub mod state;
pub mod storage_client;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    /// The project, e.g. 'ant-data-farm'. This translates to sets of environment variables.
    pub source_project: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResponse {
    occurred_at: DateTime<Utc>,
}

async fn post_backup(
    State(AntBackingItUpState {
        root_dir,
        ant_archive,
        ..
    }): State<AntBackingItUpState>,
    Json(req): Json<BackupRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // First, backup the remote database to a local file
    let db_params: DatabaseParams = match req.source_project.as_str() {
        "ant-data-farm" => DatabaseParams {
            host: dotenv::var("ANT_DATA_FARM_HOST").expect("No ANT_DATA_FARM_HOST variable"),
            port: dotenv::var("ANT_DATA_FARM_PORT")
                .expect("No ANT_DATA_FARM_PORT variable")
                .parse::<u16>()
                .expect("port was not u16"),
            db_name: ant_library::secret::load_secret("ant_data_farm_db").unwrap(),
            username: ant_library::secret::load_secret("ant_data_farm_user").unwrap(),
            password: ant_library::secret::load_secret("ant_data_farm_password").unwrap(),
        },
        p => {
            warn!("ANT-ERR-007: Unsupported project {p}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let now = chrono::Utc::now();
    let local_sql_filename = format!(
        "{}.{}-{}-{}.{}-{}-{}.bak.sql",
        req.source_project,
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let local_sql_path = root_dir.join(&local_sql_filename);

    let mut cmd = PgDumpBuilder::new()
        .create()
        .clean()
        .if_exists()
        .serializable_deferrable()
        .dbname(&db_params.db_name)
        .username(&db_params.username)
        .pg_password(&db_params.password)
        .host(&db_params.host)
        .port(db_params.port)
        .file(&local_sql_path)
        .build();

    info!("Executing backup: {}", cmd.to_command_string());

    let out = cmd.output().map_err(|e| {
        error!("ANT-ERR-008: pg_dump execution: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let stdout = String::from_utf8(out.stdout).map_err(|e| {
        error!("ANT-ERR-009: stdout not utf8: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let stderr = String::from_utf8(out.stderr).map_err(|e| {
        error!("ANT-ERR-010: stdout not utf8: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !out.status.success() {
        error!(
            "ANT-ERR-011: pg_dump failed.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let md: std::fs::Metadata = std::fs::metadata(&local_sql_path).unwrap();

    info!(
        "Reading {} backup SQL: {}",
        humansize::format_size(md.len(), humansize::DECIMAL),
        local_sql_path.display()
    );
    let sql_plaintext = std::fs::read(&local_sql_path).unwrap();

    {
        info!("Saving plaintext to ant-archive...");
        let plaintext_bytes = Bytes::from(sql_plaintext);

        let key = format!(
            "{}pgdumpbackup.{}-{}-{}.{}-{}-{}.bak.sql.zip",
            backup_key_prefix(&req.source_project),
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );

        ant_archive
            .put_object("b-typesofants", &key, plaintext_bytes.clone())
            .await
            .context("ant-archive operation")
            .unwrap();
    }

    info!("Removing backup SQL: {}", local_sql_path.display());
    std::fs::remove_file(&local_sql_path).expect(&format!(
        "removing pg_dump output: {}",
        local_sql_path.display()
    ));

    rotation::rotate_backups(&ant_archive, "b-typesofants", &req.source_project)
        .await
        .map_err(|e| {
            error!("ANT-ERR-138: backup saved, but rotation failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Backup job successful.");

    return Ok(StatusCode::OK);
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanBackupsRequest {
    /// The project, e.g. 'ant-data-farm'. This translates to sets of environment variables.
    pub source_project: String,
}

async fn clean_backups(
    State(AntBackingItUpState { ant_archive, .. }): State<AntBackingItUpState>,
    Json(req): Json<CleanBackupsRequest>,
) -> Result<StatusCode, StatusCode> {
    rotation::rotate_backups(&ant_archive, "b-typesofants", &req.source_project)
        .await
        .map_err(|e| match e {
            RotationError::UnsupportedProject(_) => {
                warn!("ANT-ERR-139: {e}");
                StatusCode::BAD_REQUEST
            }
            e => {
                error!("ANT-ERR-140: rotation failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    return Ok(StatusCode::OK);
}

pub fn make_routes(s: AntBackingItUpState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Routes::new()
        .post("/backup", post(post_backup))
        .post("/clean", post(clean_backups))
        .build()
        .with_state(s)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(cors)
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                ))),
        );

    return Ok(app);
}
