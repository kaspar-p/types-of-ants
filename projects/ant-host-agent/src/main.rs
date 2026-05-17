use ant_host_agent::{make_routes, state::AntHostAgentState};
use std::{collections::HashMap, fs, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument};

#[tokio::main]
#[instrument(name = "ant_host_agent")]
async fn main() {
    ant_library::set_global_logs("ant-host-agent");

    info!("Initializing state...");
    let state = AntHostAgentState {
        services: Arc::new(Mutex::new(HashMap::new())),
        archive_root_dir: PathBuf::from(
            dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR variable."),
        )
        .join("fs")
        .join("archives"),
        install_root_dir: PathBuf::from(
            dotenv::var("ANT_HOST_AGENT_INSTALL_ROOT_DIR")
                .expect("No ANT_HOST_AGENT_INSTALL_ROOT_DIR variable."),
        ),
        secrets_root_dir: PathBuf::from(
            dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR variable."),
        )
        .join("fs")
        .join("secrets"),
    };

    info!("Init directory: {}", state.archive_root_dir.display());
    fs::create_dir_all(&state.archive_root_dir)
        .expect(format!("Failed mkdir: {}", state.archive_root_dir.display()).as_str());
    info!("Init directory: {}", state.install_root_dir.display());
    fs::create_dir_all(&state.install_root_dir)
        .expect(format!("Failed mkdir: {}", state.install_root_dir.display()).as_str());

    info!("Initializing routes...");
    let api = make_routes(state.clone()).expect("init api");

    let scan_handle = {
        info!("Reading systemd status to get current list of services...");
        let state2 = state.clone();
        tokio::task::spawn(ant_host_agent::systemd::scan::find_active_services(state2))
    };

    let slice_handle = {
        info!("Ensuring typesofants.slice exists.");
        let state2 = state.clone();
        tokio::task::spawn(ant_host_agent::systemd::slice::ensure_slice(state2))
    };

    let (scan, slice) = tokio::try_join!(slice_handle, scan_handle).expect("join error");
    if let Err(scan) = scan {
        error!("Failed to scan for existing typesofants services: {scan}");
    }
    if let Err(slice) = slice {
        error!("Failed to ensure typesofants.slice exists: {slice}");
    }

    info!("Starting server...");
    let port = dotenv::var("PORT")
        .expect("Could not find PORT environment variable")
        .parse::<u16>()
        .expect("PORT environment variable needs to be a valid port!");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{}]...",
        ant_library::get_mode(),
        addr.to_string()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind to {port}").as_str());

    axum::serve(listener, api).await.expect("server failed");
}
