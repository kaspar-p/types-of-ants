use std::io::{Read, Write};

use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, KeyInit,
};
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Datelike, Timelike, Utc};
use http::{header, Method, StatusCode};
use postgresql_commands::{pg_dump::PgDumpBuilder, traits::CommandToString, CommandBuilder};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info, warn};
use zip::write::SimpleFileOptions;

use crate::{
    state::AntBackingItUpState,
    storage_client::{Backup, DatabaseParams},
};

pub mod state;
pub mod storage_client;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBackupsRequest {
    pub page: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBackupsResponse {
    pub backups: Vec<Backup>,
}

async fn list_backups(
    State(AntBackingItUpState { db, .. }): State<AntBackingItUpState>,
) -> Result<impl IntoResponse, StatusCode> {
    let backups = db.get_all_backups().await.map_err(|e| {
        error!("db: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    return Ok((StatusCode::OK, Json(ListBackupsResponse { backups })));
}

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
        mut db,
        mut ant_fs,
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
            warn!("Unsupported project {p}");
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
        error!("pg_dump execution: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let stdout = String::from_utf8(out.stdout).map_err(|e| {
        error!("stdout not utf8: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let stderr = String::from_utf8(out.stderr).map_err(|e| {
        error!("stdout not utf8: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !out.status.success() {
        error!("pg_dump failed.\nstdout: {}\nstderr: {}", stdout, stderr);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Reading backup SQL: {}", local_sql_path.display());
    let mut sql_plaintext: Vec<u8> = vec![];
    std::fs::File::open(&local_sql_path)
        .unwrap()
        .read_to_end(&mut sql_plaintext)
        .unwrap();
    info!("Removing backup SQL: {}", local_sql_path.display());
    std::fs::remove_file(&local_sql_path).expect(&format!(
        "removing pg_dump output: {}",
        local_sql_path.display()
    ));

    let local_zip_filename = format!("{local_sql_filename}.zip");
    let local_zip_path = root_dir.join(&local_zip_filename);
    let local_zip_file = std::fs::File::create(&local_zip_path).unwrap();
    info!("Creating zip archive: {}", local_zip_path.display());
    let mut zip = zip::ZipWriter::new(local_zip_file);

    info!(
        "Writing {} bytes to zip archive: {}",
        sql_plaintext.len(),
        local_zip_path.display()
    );
    zip.start_file(
        local_sql_filename,
        SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Xz)
            .large_file(true),
    )
    .expect("zip create file");
    zip.write_all(&sql_plaintext).expect("zip write");
    zip.finish().expect("zip flush");
    let mut plaintext_zip_file = std::fs::File::open(&local_zip_path).expect("open zip file");

    info!("Reading zip archive: {}", local_zip_path.display());
    let mut plaintext_zip_buf: Vec<u8> = vec![];
    plaintext_zip_file
        .read_to_end(&mut plaintext_zip_buf)
        .expect("plaintext zip read");

    info!("Deleting zip archive: {}", local_zip_path.display());
    std::fs::remove_file(&local_zip_path)
        .expect(&format!("removing zip file: {}", local_zip_path.display()));

    info!("Encrypting zip archive...");
    let cipher = Aes256Gcm::new((&[0; 32]).into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // Unique per file

    let ciphertext = cipher.encrypt(&nonce, plaintext_zip_buf.as_ref()).unwrap();

    // Then save that file to an ant-fs worker.
    info!("Saving to ant-fs...");
    let remote_filename = format!("{}.enc", &local_zip_filename);

    ant_fs.put_file(&remote_filename, ciphertext).await.unwrap();

    // Then save all of that in the database.
    info!("Recording backup job...");
    db.record_backup(
        &req.source_project,
        &db_params,
        &nonce.to_vec(),
        &ant_fs.host,
        ant_fs.port,
        &remote_filename,
    )
    .await
    .map_err(|e| {
        error!("db query failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("Backup job successful.");

    return Ok(StatusCode::OK);
}

pub fn make_routes(s: AntBackingItUpState) -> Result<Router, anyhow::Error> {
    debug!("Initializing API route...");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    debug!("Initializing site routes...");
    let app = Router::new()
        .route("/backups", get(list_backups))
        .route("/backup", post(post_backup))
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
