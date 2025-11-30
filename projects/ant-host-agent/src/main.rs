use ant_host_agent::{make_routes, state::AntHostAgentState};
use std::{fs, net::SocketAddr, path::PathBuf};
use tracing::{debug, info};

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-host-agent");

    info!("Initializing state...");
    let state = AntHostAgentState {
        archive_root_dir: PathBuf::from(
            PathBuf::from(dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR variable."))
                .join("fs")
                .join("archives"),
        ),
        install_root_dir: PathBuf::from(
            dotenv::var("ANT_HOST_AGENT_INSTALL_ROOT_DIR")
                .expect("No ANT_HOST_AGENT_INSTALL_ROOT_DIR variable."),
        ),
        secrets_root_dir: PathBuf::from(
            PathBuf::from(dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR variable."))
                .join("fs")
                .join("secrets"),
        ),
    };

    info!("Init directory: {}", state.archive_root_dir.display());
    fs::create_dir_all(&state.archive_root_dir)
        .expect(format!("Failed mkdir: {}", state.archive_root_dir.display()).as_str());
    info!("Init directory: {}", state.install_root_dir.display());
    fs::create_dir_all(&state.install_root_dir)
        .expect(format!("Failed mkdir: {}", state.install_root_dir.display()).as_str());

    info!("Initializing routes...");
    let api = make_routes(state).await.expect("init api");

    info!("Starting server...");
    let port = dotenv::var("ANT_HOST_AGENT_PORT")
        .expect("Could not find ANT_HOST_AGENT_PORT environment variable")
        .parse::<u16>()
        .expect("ANT_HOST_AGENT_PORT environment variable needs to be a valid port!");

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
