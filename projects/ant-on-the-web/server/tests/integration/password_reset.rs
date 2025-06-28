use std::any::Any;

use ant_on_the_web::users::{
    GetUserResponse, LoginRequest, PasswordRequest, PasswordResetCodeRequest,
    PasswordResetSecretRequest, PasswordResetSecretResponse,
};
use http::StatusCode;
use tracing_test::traced_test;

use crate::{
    fixture::{test_router_auth, test_router_no_auth, TestSmsSender},
    fixture_sms::{second_otp, third_otp},
};

#[tokio::test]
#[traced_test]
async fn users_password_reset_code_returns_400_if_phone_number_or_invalid() {
    let fixture = test_router_no_auth().await;

    {
        let req = PasswordResetCodeRequest {
            username: "user".to_string(),
            phone_number: "not a phone number".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_code_returns_200_and_sends_no_messages_if_user_does_not_exist() {
    let fixture = test_router_no_auth().await;

    {
        let req = PasswordResetCodeRequest {
            username: "not a user".to_string(),
            phone_number: "+12223334444".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let sms = fixture.state.sms.as_ref() as &dyn Any;
        let sms: &TestSmsSender = sms.downcast_ref().unwrap();

        assert_eq!(sms.all_msgs().await.len(), 0);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_code_returns_200_and_sends_code_if_user_exists() {
    // Use authenticated router just to create a fake 'user' user.
    let (fixture, cookie) = test_router_auth().await;

    let (username, phone_number) = {
        let res = fixture
            .client
            .get("/api/users/user")
            .header("Cookie", &cookie)
            .send()
            .await;
        let res: GetUserResponse = res.json().await;

        (
            res.user.username.clone(),
            res.user.phone_numbers.get(0).unwrap().clone(),
        )
    };

    {
        let req = PasswordResetCodeRequest {
            username: username,
            phone_number: phone_number,
        };
        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let sms = fixture.state.sms.as_ref() as &dyn Any;
        let sms: &TestSmsSender = sms.downcast_ref().unwrap();

        let msgs = sms.all_msgs().await;
        assert_eq!(msgs.len(), 2); // one for the initial user signup, one just now
        assert_eq!(msgs.get(1).unwrap().to_phone, "+11112223333");
        assert_eq!(
            msgs.get(1).unwrap().content,
            format!("[typesofants.org] your one-time code is: {}", third_otp())
        );
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_code_returns_200_and_cancels_outstanding_otp_requests() {
    let (fixture, _) = test_router_auth().await;

    // Step 1
    {
        let req = PasswordResetCodeRequest {
            username: "user".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // step 2: should cancel outstanding reqs
    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: third_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: PasswordResetSecretResponse = res.json().await;
        assert!(res.secret.contains(".")); // some valid jwt
    }

    // step 2 again: should no longer work
    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: third_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_secret_returns_400_if_otp_is_wrong() {
    let fixture = test_router_no_auth().await;

    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: "wrong".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_secret_returns_400_if_otp_is_cancelled() {
    let fixture = test_router_no_auth().await;

    for _ in 0..10 {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: "wrong".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // Correct one fails even now
    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: second_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_secret_returns_400_if_otp_is_already_verified() {
    // Use this just to get a user created.
    let (fixture, _) = test_router_auth().await;

    // Correct one so it cancels the previous
    {
        let req = PasswordResetCodeRequest {
            username: "user".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: third_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    };

    // Trying the correct code again returns 400
    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: second_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_reset_secret_returns_200_with_secret_if_otp_is_correct() {
    // Use this just to get a user created.
    let (fixture, _) = test_router_auth().await;

    // Correct one so it cancels the previous
    {
        let req = PasswordResetCodeRequest {
            username: "user".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: third_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: PasswordResetSecretResponse = res.json().await;
        assert!(res.secret.contains(".")); // some valid jwt
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_returns_400_if_password_attempts_do_not_match() {
    let fixture = test_router_no_auth().await;

    {
        let req = PasswordRequest {
            password1: "ant-password1".to_string(),
            password2: "ant-password2".to_string(),
            secret: "".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_returns_400_if_password_attempts_are_not_valid_passwords() {
    let fixture = test_router_no_auth().await;

    // password1 not valid
    {
        let req = PasswordRequest {
            password1: "".to_string(),
            password2: "ant-password2".to_string(),
            secret: "".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // password2 not valid
    {
        let req = PasswordRequest {
            password1: "ant-password1".to_string(),
            password2: "".to_string(),
            secret: "".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // passwords match but are invalid
    {
        let req = PasswordRequest {
            password1: "pass".to_string(),
            password2: "pass".to_string(),
            secret: "".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_returns_401_if_secret_is_wrong_or_tampered() {
    let fixture = test_router_no_auth().await;
    {
        let req = PasswordRequest {
            password1: "ant-password1".to_string(),
            password2: "ant-password1".to_string(),
            secret: "something that is not a valid jwt".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_returns_200_and_resets_password() {
    // Use this just to get a user created.
    let (fixture, _) = test_router_auth().await;

    // step 1
    {
        let req = PasswordResetCodeRequest {
            username: "user".to_string(),
            phone_number: "+1 (111) 222-3333".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-code")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // step 2
    let secret = {
        let req = PasswordResetSecretRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            otp: third_otp(),
        };

        let res = fixture
            .client
            .post("/api/users/password-reset-secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: PasswordResetSecretResponse = res.json().await;
        assert!(res.secret.contains(".")); // some valid jwt

        res.secret
    };

    // step 3
    {
        let req = PasswordRequest {
            password1: "ant-password1".to_string(),
            password2: "ant-password1".to_string(),
            secret: secret,
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // try to login with that new password
    {
        let req = LoginRequest {
            method: ant_on_the_web::users::LoginMethod::Username("user".to_string()),
            password: "ant-password1".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn users_password_returns_200_if_authenticated_with_no_secret() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = PasswordRequest {
            password1: "ant-password1".to_string(),
            password2: "ant-password1".to_string(),
            secret: "".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/password")
            .json(&req)
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // try to login with that new password
    {
        let req = LoginRequest {
            method: ant_on_the_web::users::LoginMethod::Username("user".to_string()),
            password: "ant-password1".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}
