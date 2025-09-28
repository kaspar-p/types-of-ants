use std::{fs::create_dir_all, net::SocketAddr, path::PathBuf, time::Duration};

use ant_backing_it_up::{
    state::AntBackingItUpState,
    storage_client::{AntBackingItUpStorageClient, DatabaseParams},
    BackupRequest,
};
use ant_fs_client::{AntFsClient, AntFsHostPorts};
use futures::future::join_all;
use stdext::prelude::DurationExt;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-backing-it-up");

    debug!("Setting up state...");

    let persist_dir = dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR environment variable!");
    let root_path = dotenv::var("ANT_BACKING_IT_UP_ROOT_PATH")
        .expect("No ANT_BACKING_IT_UP_ROOT_PATH environment variable!");

    let root_dir = PathBuf::from(persist_dir).join(root_path);
    create_dir_all(&root_dir).expect("failed to create root dir");

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

    let ant_fs_host_ports = serde_json::from_str::<AntFsHostPorts>(
        &dotenv::var("ANT_FS_HOST_PORTS").expect("no ANT_FS_HOST_PORTS variable"),
    )
    .expect("malformed ANT_FS_HOST_PORTS");

    let ant_fs = AntFsClient::new(
        ant_fs_host_ports[0].url.split(":").collect::<Vec<&str>>()[0],
        ant_fs_host_ports[0].url.split(":").collect::<Vec<&str>>()[1]
            .parse::<u16>()
            .unwrap(),
        ant_library::secret::load_secret("ant_fs_client_creds")
            .unwrap()
            .split("\n")
            .collect::<Vec<&str>>()[0]
            .split(":")
            .collect::<Vec<&str>>()[0]
            .to_string(),
        ant_library::secret::load_secret("ant_fs_client_creds")
            .unwrap()
            .split("\n")
            .collect::<Vec<&str>>()[0]
            .split(":")
            .collect::<Vec<&str>>()[1]
            .to_string(),
        ant_fs_host_ports[0].tls,
    );

    let state = AntBackingItUpState {
        root_dir,
        db,
        ant_fs,
    };

    let app = ant_backing_it_up::make_routes(state).expect("failed to init api");

    let port: u16 = dotenv::var("ANT_BACKING_IT_UP_PORT")
        .expect("ANT_BACKING_IT_UP_PORT environment variable not found")
        .parse()
        .expect("ANT_BACKING_IT_UP_PORT was not u16");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{}]...",
        ant_library::get_mode(),
        addr.to_string()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());

    let api_handle = tokio::spawn(async {
        axum::serve(listener, app).await.expect("server failed");
    });

    let cron_handle = tokio::spawn(async move {
        let client = reqwest::Client::new();

        loop {
            info!("Creating periodic backup...");

            for project in vec!["ant-data-farm"] {
                let res = client
                    .post(format!("http://{}:{}/backup", addr.ip(), addr.port()))
                    .json(&BackupRequest {
                        source_project: project.to_string(),
                    })
                    .send()
                    .await;

                match res {
                    Ok(res) => {
                        info!("backup {} status: {}", project, res.status());
                    }
                    Err(e) => {
                        error!("backup {} failed: {}", project, e);
                    }
                }
            }

            sleep(Duration::ZERO.add_hours(1)).await;
        }
    });

    join_all(vec![api_handle, cron_handle]).await;
}
