use std::{path::PathBuf, sync::Arc};

use ant_data_farm::{AntDataFarmClient, DatabaseConfig, DatabaseCredentials};
use ant_library::axum_test_client::TestClient;
use ant_on_the_web::{
    make_routes,
    users::{LoginMethod, LoginRequest, LoginResponse, SignupRequest},
};
use http::StatusCode;
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

/// Get a test webserver connected to a test webserver.
/// The database has been bootstrapped with the most modern schema.
pub async fn test_router() -> TestFixture {
    let (db, db_client) = test_database_client().await;
    let app = make_routes(Arc::new(db_client)).unwrap();
    return TestFixture {
        client: TestClient::new(app).await,
        _guard: db,
    };
}

/// Get a test webserver and database, along with a valid COOKIE header value.
pub async fn authn_test_router() -> (TestFixture, String) {
    let fixture = test_router().await;

    {
        let req = SignupRequest {
            username: "user".to_string(),
            email: "email@domain.com".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
            password: "my-ant-password".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "Signup completed.");
    }

    let token = {
        let req = LoginRequest {
            method: LoginMethod::Username("user".to_string()),
            password: "my-ant-password".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let res: LoginResponse = res.json().await;
        res.access_token
    };

    return (fixture, format!("typesofants_auth={}", token));
}
