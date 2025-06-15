use std::{path::PathBuf, sync::Arc};

use ant_data_farm::{AntDataFarmClient, DatabaseConfig, DatabaseCredentials};
use ant_library::axum_test_client::TestClient;
use ant_on_the_web::{
    make_routes,
    sms::SmsSender,
    state::InnerApiState,
    users::{
        LoginMethod, LoginRequest, LoginResponse, SignupRequest, VerificationAttemptRequest,
        VerificationAttemptResponse, VerificationSubmission,
    },
};
use http::StatusCode;
use postgresql_embedded::PostgreSQL;
use rand::SeedableRng;
use tokio::sync::Mutex;

use crate::fixture_sms::first_sms_otp;

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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestMsg {
    pub to_phone: String,
    pub content: String,
}

pub struct TestSmsSender {
    pub msgs: Arc<Mutex<Vec<TestMsg>>>,
}

impl TestSmsSender {
    pub async fn all_msgs(&self) -> Vec<TestMsg> {
        let msgs = self.msgs.lock().await;

        return msgs.iter().map(|m| m.clone()).collect::<Vec<TestMsg>>();
    }
}

#[async_trait::async_trait]
impl SmsSender for TestSmsSender {
    async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, anyhow::Error> {
        let mut msgs = self.msgs.lock().await;
        msgs.push(TestMsg {
            to_phone: to_phone.to_string(),
            content: content.to_string(),
        });

        Ok("send-id".to_string())
    }
}

pub struct TestFixture {
    pub client: TestClient,
    pub state: InnerApiState,
    _guard: PostgreSQL,
}

async fn test_router_seeded_no_auth(seed: [u8; 32]) -> TestFixture {
    let (db, db_client) = test_database_client().await;
    let sms = TestSmsSender {
        msgs: Arc::new(Mutex::new(vec![])),
    };

    let state = InnerApiState {
        dao: Arc::new(db_client),
        sms: Arc::new(sms),

        // Deterministic seed for testing.
        rng: Arc::new(Mutex::new(rand::rngs::StdRng::from_seed(seed))),
    };
    let app = make_routes(&state).unwrap();

    return TestFixture {
        client: TestClient::new(app).await,
        state: state,
        _guard: db,
    };
}

/// Get a test webserver connected to a test webserver.
/// The database has been bootstrapped with the most modern schema.
pub async fn test_router_no_auth() -> TestFixture {
    test_router_seeded_no_auth([123; 32]).await
}

/// Get a test webserver and database, along with a valid COOKIE header value.
pub async fn test_router_auth() -> (TestFixture, String) {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    let token = {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: VerificationAttemptResponse = res.json().await;
        res.token
    };

    return (fixture, format!("typesofants_auth={}", token));
}

/// Get a router and cookie pair that has not perform 2fa verification yet.
pub async fn test_router_weak_auth(seed: Option<[u8; 32]>) -> (TestFixture, String) {
    let fixture = test_router_seeded_no_auth(seed.unwrap_or([123; 32])).await;

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
