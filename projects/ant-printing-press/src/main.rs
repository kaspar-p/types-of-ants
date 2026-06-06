use std::net::SocketAddr;

use ant_library::get_mode;
use tracing::debug;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-printing-press");

    debug!("Setting up state...");
    let state = ant_printing_press::state::AntPrintingPressState {};

    let app = ant_printing_press::make_routes(&state).expect("api init");

    let port = dotenv::var("PRIMARY_PORT")
        .expect("No PRIMARY_PORT variable!")
        .parse::<u16>()
        .expect("PRIMARY_PORT was not u16!");

    debug!("Starting [{}] server on port [{}]...", get_mode(), port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}
