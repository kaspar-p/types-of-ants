mod common;
mod procs;
mod routes;

use ant_host_agent::start_server;

#[tokio::main]
async fn main() -> () {
    start_server(Some(7008))
        .await
        .expect("Server failed to start!");

    ()
}
