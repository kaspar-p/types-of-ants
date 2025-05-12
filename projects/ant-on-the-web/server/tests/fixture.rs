use std::{path::PathBuf, sync::Arc};

use ant_data_farm::{AntDataFarmClient, DatabaseConfig, DatabaseCredentials};
use ant_library::axum_test_client::TestClient;
use ant_on_the_web::make_routes;
use postgresql_embedded::PostgreSQL;

async fn test_database_client() -> (PostgreSQL, AntDataFarmClient) {
    let mut pg = PostgreSQL::new(postgresql_embedded::Settings {
        temporary: true,
        ..Default::default()
    });
    pg.setup().await.unwrap();
    pg.start().await.unwrap();

    pg.create_database("test").await.unwrap();

    let client = AntDataFarmClient::new(Some(DatabaseConfig {
        port: Some(pg.settings().port),
        host: Some(pg.settings().host.clone()),
        creds: Some(DatabaseCredentials {
            database_name: "test".to_string(),
            database_password: pg.settings().password.clone(),
            database_user: pg.settings().username.clone(),
        }),
        migration_dir: Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("ant-data-farm/data/sql"),
        ),
    }))
    .await
    .expect("connection failed");

    return (pg, client);
}

pub struct TestFixture {
    pub client: TestClient,
    _guard: PostgreSQL,
}

pub async fn test_router() -> TestFixture {
    let (db, db_client) = test_database_client().await;
    let app = make_routes(Arc::new(db_client)).unwrap();
    return TestFixture {
        client: TestClient::new(app).await,
        _guard: db,
    };
}
