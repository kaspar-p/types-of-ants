use std::{fs::create_dir_all, net::SocketAddr, path::PathBuf};

use ant_zookeeper::state::AntZookeeperState;
use tracing::debug;
use zbus_systemd::zbus;

#[tokio::main]
async fn main() {
    ant_library::set_global_logs("ant-zookeeper");

    debug!("Setting up state...");

    let persist_dir = dotenv::var("PERSIST_DIR").expect("No PERSIST_DIR environment variable!");
    let root_path = dotenv::var("ANT_ZOOKEEPER_ROOT_PATH")
        .expect("No ANT_ZOOKEEPER_ROOT_PATH environment variable!");

    let root_dir = PathBuf::from(persist_dir).join(root_path);
    create_dir_all(&root_dir).expect("failed to create root dir");

    let state = AntZookeeperState { root_dir };

    let app = ant_zookeeper::make_routes(state).expect("failed to init api");

    let port: u16 = dotenv::var("ANT_ZOOKEEPER_PORT")
        .expect("ANT_ZOOKEEPER_PORT environment variable not found")
        .parse()
        .expect("ANT_ZOOKEEPER_PORT was not u16");

    let conn = zbus::Connection::system().await.expect("system connection");
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .expect("manager init");

    let a = manager
        .start_unit("ant-on-the-web.service".to_string(), "mode".to_string())
        .await
        .unwrap();
    println!("{}", a);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    debug!(
        "Starting [{}] server on [{}]...",
        ant_library::get_mode(),
        addr.to_string()
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("failed to bind server to {port}").as_str());
    axum::serve(listener, app).await.expect("server failed");
}
