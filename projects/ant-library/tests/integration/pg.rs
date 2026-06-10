use std::sync::Arc;

use ant_library::sd::{
    pg::{make_connection_string, DynamicPostgresManager},
    reader::ServiceDiscovery,
    writer::ServiceDiscoveryWriter,
};
use ant_library_test::{consul_fixture::ConsulFixture, db::TestDatabase};
use postgresql_embedded::Settings;
use tokio_postgres::NoTls;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn pool_recycles_on_endpoint_change() {
    let ip = local_ip_address::local_ip().unwrap().to_string();

    let db1 = TestDatabase::with_settings(
        "ant-data-farm",
        Settings {
            host: "127.0.0.1".to_string(),
            password: "password1".to_string(),
            ..Default::default()
        },
    )
    .await;
    let db2 = TestDatabase::with_settings(
        "ant-data-farm",
        Settings {
            host: "127.0.0.1".to_string(),
            password: "password1".to_string(),
            ..Default::default()
        },
    )
    .await;

    assert_eq!(db1.config.database_name, db2.config.database_name);
    assert_eq!(db1.config.database_user, db2.config.database_user);
    assert_eq!(db1.config.database_password, db2.config.database_password);
    assert_ne!(db1.config.port, db2.config.port);

    let consul = ConsulFixture::new().await;

    {
        ServiceDiscoveryWriter::new(consul.port())
            .register_remote_service("ant-data-farm", "127.0.0.1", db1.config.port)
            .await
            .unwrap();
    }

    let sd = Arc::new(ServiceDiscovery::new(consul.port()));
    let manager = DynamicPostgresManager::new_dynamic(
        sd.clone(),
        "ant-data-farm",
        db1.config.database_name.clone(),
        db1.config.database_user.clone(),
        db1.config.database_password.clone(),
    );

    let pool = bb8::Pool::builder()
        .max_size(2)
        .build(manager)
        .await
        .unwrap();

    // Connections go to pg1
    let conn = pool.get().await.unwrap();
    conn.query_one("SELECT 1", &[]).await.unwrap();
    assert_eq!(conn.port, db1.config.port);

    // Remove db1 from connection list
    {
        drop(db1);
    }

    // Switch endpoint to db2
    {
        ServiceDiscoveryWriter::new(consul.port())
            .register_remote_service("ant-data-farm", "127.0.0.1", db2.config.port)
            .await
            .unwrap();
    }

    // is_valid rejects idle conns, pool creates fresh connection to pg2
    let conn = pool.get().await.unwrap();
    conn.query_one("SELECT 1", &[]).await.unwrap();
    assert_eq!(conn.port, db2.config.port);
}
