use ant_host_agent::make_routes;
use std::net::SocketAddr;
use tracing::{debug, info};

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-host-agent");

    info!("Initializing routes...");
    let api = make_routes().await.expect("init api");

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
