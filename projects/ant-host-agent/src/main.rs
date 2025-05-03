mod common;
mod procs;
mod routes;

use ant_host_agent::start_server;
use ant_library::set_global_logs;
use tracing::info;

#[tokio::main]
async fn main() -> () {
    set_global_logs("ant-host-agent");

    info!("Initializing...");
    start_server(None).await.expect("Server failed to start!");

    ()
}
