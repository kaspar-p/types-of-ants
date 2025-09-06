use std::net::SocketAddr;

use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-fs");

    debug!("Setting up state...");
    let app = ant_fs::make_routes().expect("failed to init api");

    let port: u16 = dotenv::var("ANT_FS_PORT")
        .expect("ANT_FS_PORT environment variable not found")
        .parse()
        .expect("ANT_FS_PORT was not u16");

    debug!(
        "Starting [{}] server on port [{}]...",
        ant_library::get_mode(),
        port
    );
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}
