use std::path::PathBuf;

use ant_backing_it_up::{
    crypto,
    storage_client::{AntBackingItUpStorageClient, DatabaseParams},
};
use ant_fs_client::AntFsClient;
use clap::{command, Parser};
use tracing::info;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    project: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    ant_library::set_global_logs("fetch-backup");

    dotenv::from_path(
        PathBuf::from(dotenv::var("TYPESOFANTS_SECRET_DIR").unwrap()).join("build.cfg"),
    )
    .unwrap();

    let db = AntBackingItUpStorageClient::connect(&DatabaseParams {
        db_name: ant_library::secret::load_secret("ant_backing_it_up_storage_db").unwrap(),
        username: ant_library::secret::load_secret("ant_backing_it_up_storage_user").unwrap(),
        password: ant_library::secret::load_secret("ant_backing_it_up_storage_password").unwrap(),
        host: dotenv::var("ANT_BACKING_IT_UP_STORAGE_HOST")
            .expect("No ANT_BACKING_IT_UP_STORAGE_HOST variable."),
        port: dotenv::var("ANT_BACKING_IT_UP_STORAGE_PORT")
            .expect("No ANT_BACKING_IT_UP_STORAGE_PORT variable.")
            .parse::<u16>()
            .expect("port was not u16"),
    })
    .await
    .expect("db param");

    let ant_fs_creds = ant_library::secret::load_secret("ant_fs_client_creds").unwrap();
    let ant_fs_creds = ant_fs_creds
        .split("\n")
        .collect::<Vec<&str>>()
        .first()
        .unwrap()
        .split(":")
        .collect::<Vec<&str>>();
    let username = ant_fs_creds[0];
    let password = ant_fs_creds[1];

    let backup = db
        .get_latest_backup_for_project(&args.project)
        .await
        .expect("list backups")
        .unwrap_or_else(|| panic!("Project has never had a backup!"));

    info!(
        "Retrieving backup {} @ {} on [{}:{}/{}]",
        backup.project,
        backup.created_at,
        backup.destination_host,
        backup.destination_port,
        backup.destination_filepath
    );

    let ant_fs = AntFsClient::new(
        &backup.destination_host,
        backup.destination_port,
        username.to_string(),
        password.to_string(),
        false,
    );

    let ciphertext_bytes = ant_fs
        .get_file(&backup.destination_filepath)
        .await
        .expect("GET")
        .unwrap();

    info!(
        "Decrypting with nonce {}...",
        hex::encode(&backup.encryption_nonce)
    );

    let plaintext = crypto::decrypt_backup(&backup.encryption_nonce, &ciphertext_bytes);

    let sql_filepath = backup
        .destination_filepath
        .split(".zip.enc")
        .collect::<Vec<&str>>();
    let sql_filepath = sql_filepath.first().unwrap();
    let local_filepath = PathBuf::from(".")
        .join("restore")
        .join(dotenv::var("TYPESOFANTS_ENV").unwrap())
        .join(sql_filepath);
    std::fs::create_dir_all(local_filepath.parent().unwrap()).unwrap();

    info!("Retrieving backup to {}", local_filepath.display());

    let mut reader = zip::ZipArchive::new(std::io::Cursor::new(plaintext)).unwrap();
    let mut zip_file = reader.by_name(&sql_filepath).unwrap();

    let mut output = std::fs::File::create(local_filepath).unwrap();

    std::io::copy(&mut zip_file, &mut output).unwrap();

    info!("Retrieved!");
}
