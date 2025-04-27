mod common;
mod procs;
mod routes;

use ant_host_agent::start_server;

#[tokio::main]
async fn main() -> () {
    start_server(None).await.expect("Server failed to start!");

    ()
}
