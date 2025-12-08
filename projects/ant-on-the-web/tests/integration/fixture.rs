use std::{env::set_var, path::PathBuf, sync::Arc};

use ant_data_farm::AntDataFarmClient;
use ant_library::{
    axum_test_client::TestClient,
    db::{fixture::test_database_config, TypesOfAntsDatabase},
};
use ant_on_the_web::{
    make_routes,
    sms::{SmsError, SmsSender},
    state::InnerApiState,
    users::{
        AddEmailRequest, AddEmailResponse, AddPhoneNumberRequest, AddPhoneNumberResponse,
        AddResolution, GetUserResponse, LoginMethod, LoginRequest, LoginResponse, SignupRequest,
        VerificationAttemptRequest, VerificationSubmission,
    },
    ApiOptions,
};
use http::{header::SET_COOKIE, HeaderMap, StatusCode};
use postgresql_embedded::PostgreSQL;
use rand::SeedableRng;
use tokio::sync::Mutex;

use crate::fixture_sms::first_otp;
use crate::{fixture_email::TestEmailSender, fixture_sms::second_otp};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestMsg {
    pub to_phone: String,
    pub content: String,
}

pub struct TestSmsSender {
    msgs: Arc<Mutex<Vec<TestMsg>>>,
}

impl TestSmsSender {
    pub async fn all_msgs(&self) -> Vec<TestMsg> {
        let msgs = self.msgs.lock().await;

        return msgs.iter().map(|m| m.clone()).collect::<Vec<TestMsg>>();
    }
}

#[async_trait::async_trait]
impl SmsSender for TestSmsSender {
    async fn send_msg(&self, to_phone: &str, content: &str) -> Result<String, SmsError> {
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

pub struct FixtureOptions {
    enable_throttle: bool,
    seed: [u8; 32],
}

impl FixtureOptions {
    pub fn new() -> Self {
        FixtureOptions {
            enable_throttle: false,
            seed: [123u8; 32],
        }
    }

    pub fn with_throttle(mut self) -> Self {
        self.enable_throttle = true;

        self
    }
}

pub fn get_auth_cookie(headers: &HeaderMap) -> String {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .find(|h| h.to_str().unwrap().contains("typesofants_auth"))
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

pub fn get_telemetry_cookie(headers: &HeaderMap) -> String {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .find(|h| h.to_str().unwrap().contains("typesofants_telemetry"))
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

async fn test_router_seeded_no_auth(opts: FixtureOptions) -> TestFixture {
    unsafe { set_var("TYPESOFANTS_SECRET_DIR", "./tests/integration/test-secrets") };

    let (db, db_config) = test_database_config("ant-data-farm").await;
    let sms = TestSmsSender {
        msgs: Arc::new(Mutex::new(vec![])),
    };

    let state = InnerApiState {
        static_dir: PathBuf::from("./tests/integration/test-static"),

        dao: Arc::new(AntDataFarmClient::connect(&db_config).await.unwrap()),
        sms: Arc::new(sms),
        email: Arc::new(TestEmailSender::new()),

        // Deterministic seed for testing.
        rng: Arc::new(Mutex::new(rand::rngs::StdRng::from_seed(opts.seed))),
    };
    let app = make_routes(
        &state,
        ApiOptions {
            tps: match opts.enable_throttle {
                true => 25,
                false => 999_999,
            },
        },
    )
    .unwrap();

    return TestFixture {
        client: TestClient::new(app).await,
        state: state,
        _guard: db,
    };
}

/// Get a test webserver connected to a test webserver.
/// The database has been bootstrapped with the most modern schema.
pub async fn test_router_no_auth(opts: FixtureOptions) -> TestFixture {
    test_router_seeded_no_auth(opts).await
}

/// Get a test webserver and database, along with a valid COOKIE header value.
pub async fn test_router_auth(opts: FixtureOptions) -> (TestFixture, String) {
    let (fixture, cookie) = test_router_weak_auth(opts).await;

    {
        let req = AddPhoneNumberRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: AddPhoneNumberResponse = res.json().await;
        assert_eq!(body.resolution, AddResolution::Added);
    };

    let cookie = {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: first_otp(),
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

        get_auth_cookie(res.headers())
    };

    {
        let req = AddEmailRequest {
            email: "email@domain.com".to_string(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/email")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: AddEmailResponse = res.json().await;
        assert_eq!(body.resolution, AddResolution::Added);
    };

    let cookie = {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Email {
                email: "email@domain.com".to_string(),
                otp: second_otp(),
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

        get_auth_cookie(res.headers())
    };

    return (fixture, cookie);
}

/// Get a router and cookie pair that has not perform 2fa verification yet.
pub async fn test_router_weak_auth(opts: FixtureOptions) -> (TestFixture, String) {
    let fixture = test_router_seeded_no_auth(opts).await;

    {
        let req = SignupRequest {
            username: "user".to_string(),
            password: "my-ant-password".to_string(),
            password2: "my-ant-password".to_string(),
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

pub async fn test_router_admin_auth(opts: FixtureOptions) -> (TestFixture, String) {
    let (fixture, cookie) = test_router_auth(opts).await;

    let user = {
        let res = fixture
            .client
            .get("/api/users/user")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: GetUserResponse = res.json().await;

        body.user
    };

    fixture
        .state
        .dao
        .users
        .write()
        .await
        .change_user_role(&user.user_id, "admin")
        .await
        .unwrap();

    (fixture, cookie)
}
