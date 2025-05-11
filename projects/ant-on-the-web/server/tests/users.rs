use std::{path::PathBuf, sync::Arc};

use ant_data_farm::{AntDataFarmClient, DatabaseConfig, DatabaseCredentials};
use ant_library::axum_test_client::TestClient;
use http::StatusCode;
use postgresql_embedded::PostgreSQL;
use tracing_test::traced_test;

use ant_on_the_web::{make_routes, users::SignupRequest};

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

struct TestFixture {
    pub client: TestClient,
    _guard: PostgreSQL,
}

async fn test_router() -> TestFixture {
    let (db, db_client) = test_database_client().await;
    let app = make_routes(Arc::new(db_client)).unwrap();
    return TestFixture {
        client: TestClient::new(app).await,
        _guard: db,
    };
}

#[tokio::test]
async fn user_signup_fails_if_not_json() {
    let fixture = test_router().await;

    let res = fixture
        .client
        .post("/api/users/signup")
        .header("Content-Type", mime::TEXT.as_str())
        .body("text here")
        .send()
        .await;

    assert!(res.status().is_client_error());
}

#[tokio::test]
async fn user_signup_fails_if_username_invalid() {
    let fixture = test_router().await;

    {
        let req = SignupRequest {
            username: "".to_string(),
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

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Field username must be between 3 and 16 characters."
        );
    }

    {
        let req = SignupRequest {
            username: "reallylongusernamethatbreaksthevalidationcode".to_string(),
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

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Field username must be between 3 and 16 characters."
        );
    }

    {
        let req = SignupRequest {
            username: "OtherCharacters-_*090][]".to_string(),
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

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Field username must be between 3 and 16 characters."
        );
    }
}

#[tokio::test]
async fn user_signup_fails_if_phone_invalid() {
    let fixture = test_router().await;

    let req = SignupRequest {
        username: "user".to_string(),
        email: "email@domain.com".to_string(),
        phone_number: "not a phone number".to_string(),
        password: "my-ant-password".to_string(),
    };
    let res = fixture
        .client
        .post("/api/users/signup")
        .json(&req)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_eq!(res.text().await, "Field phone_number invalid.");
}

#[tokio::test]
async fn user_signup_fails_if_password_invalid() {
    let fixture = test_router().await;

    {
        let req = SignupRequest {
            username: "user".to_string(),
            email: "email@domain.com".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
            password: "my-password".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(res.text().await, "Field password must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly.");
    }

    {
        let req = SignupRequest {
            username: "user".to_string(),
            email: "email@domain.com".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
            password: "1234567".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Field password must be between 8 and 64 characters."
        );
    }

    {
        let req = SignupRequest {
            username: "user".to_string(),
            email: "email@domain.com".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
            password: "four".repeat(100).to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.text().await,
            "Field password must be between 8 and 64 characters."
        );
    }
}

#[tokio::test]
#[traced_test]
async fn user_signup_fails_if_user_already_exists() {
    let fixture = test_router().await;

    {
        let req = SignupRequest {
            username: "nobody".to_string(),
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

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(res.text().await, "User already exists.");
    }

    {
        let req = SignupRequest {
            username: "newuser".to_string(),
            email: "nobody@typesofants.org".to_string(), // the 'nobody' user email
            phone_number: "+1 (111) 222-3333".to_string(),
            password: "my-ant-password".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(res.text().await, "User already exists.");
    }

    {
        let req = SignupRequest {
            username: "newuser".to_string(),
            email: "email@domain.org".to_string(),
            phone_number: "+1 (222) 333-4444".to_string(), // the 'nobody' user phone number
            password: "my-ant-password".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(res.text().await, "User already exists.");
    }
}

#[tokio::test]
#[traced_test]
async fn user_signup_succeeds() {
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
}

#[tokio::test]
#[traced_test]
async fn user_signup_fails_if_user_already_signed_up() {
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

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(res.text().await, "User already exists.");
    }
}
