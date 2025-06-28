use std::any::Any;

use crate::{
    fixture::{test_router_auth, test_router_no_auth, test_router_weak_auth, TestSmsSender},
    fixture_sms::{first_sms_otp, second_sms_otp},
};
use ant_on_the_web::users::{
    AddPhoneNumberRequest, AddPhoneNumberResolution, AddPhoneNumberResponse, GetUserResponse,
    LoginRequest, SignupRequest, VerificationAttemptRequest, VerificationSubmission,
};
use http::{header::SET_COOKIE, StatusCode};
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_401_if_unauthenticated_call() {
    let fixture = test_router_no_auth().await;

    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: "otp".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", "typesofants_auth=other")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_200_with_different_cookie_headers() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    let phone = "+1 (111) 222-3333".to_string();
    {
        let req = AddPhoneNumberRequest {
            phone_number: phone.clone(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: phone.clone(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(new_cookie.contains("typesofants_auth="));
        assert!(new_cookie.contains("HttpOnly"));

        // They must be different!
        assert_ne!(new_cookie, cookie);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_200_with_different_cookie_headers_even_if_already_authn(
) {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    let phone = "+1 (111) 222-4444".to_string();
    {
        let req = AddPhoneNumberRequest {
            phone_number: phone.clone(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let res: AddPhoneNumberResponse = res.json().await;
        assert_eq!(res.resolution, AddPhoneNumberResolution::Added);
    }

    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: phone.clone(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let new_cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(new_cookie.contains("typesofants_auth="));
        assert!(new_cookie.contains("HttpOnly"));

        // They must be different!
        assert_ne!(new_cookie, cookie);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_400_for_unknown_phone_number() {
    let (fixture, cookie) = test_router_weak_auth(None).await;
    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-4444".to_string(),
                otp: "wrong".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_400_for_wrong_or_too_many_attempts() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    let phone = "+1 (111) 222-3333".to_string();
    {
        let req = AddPhoneNumberRequest {
            phone_number: phone.clone(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let res: AddPhoneNumberResponse = res.json().await;
        assert_eq!(res.resolution, AddPhoneNumberResolution::Added);
    }

    for _ in 0..10 {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: phone.clone(),
                otp: "wrong".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // Even the correct one fails after too many bad attempts
    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: phone.clone(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_200_after_only_signup_no_login() {
    let fixture = test_router_no_auth().await;

    let cookie = {
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

        let cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(res.text().await, "Signup completed.");

        cookie
    };

    let phone = "+1 (111) 222-3333".to_string();
    {
        let req = AddPhoneNumberRequest {
            phone_number: phone.clone(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let new_cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_ne!(new_cookie, cookie);
    }
}

/// This test fixes https://github.com/kaspar-p/types-of-ants/issues/3123 by
/// ensuring that the /phone-number endpoint that allow users to
/// associate new phone numbers only accepts weak auth if the user has never
/// associated a phone number, effectively they are not done signing up.
/// TODO: same for /email when it exists.
#[tokio::test]
#[traced_test]
async fn users_phone_number_returns_401_if_weak_auth_when_user_has_already_2fa_verified() {
    // The user has completed signup with 2fa
    let (fixture, _) = test_router_auth().await;

    let weak_auth_cookie = {
        let req = LoginRequest {
            method: ant_on_the_web::users::LoginMethod::Username("user".to_string()),
            password: "my-ant-password".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        cookie
    };

    let phone = "+1 (111) 222-3333".to_string();
    {
        let req = AddPhoneNumberRequest {
            phone_number: phone.clone(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &weak_auth_cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

/// This test fixes https://github.com/kaspar-p/types-of-ants/issues/3123 by
/// ensuring that the /phone-number endpoint that allow users to
/// associate new phone numbers only accepts weak auth if the user has never
/// associated a phone number, effectively they are not done signing up.
/// TODO: same for /email when it exists.
#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_401_if_weak_auth_when_user_has_already_2fa_verified() {
    // The user has completed signup with 2fa
    let (fixture, _) = test_router_auth().await;

    let weak_auth_cookie = {
        let req = LoginRequest {
            method: ant_on_the_web::users::LoginMethod::Username("user".to_string()),
            password: "my-ant-password".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        cookie
    };

    // requires strong auth now
    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: "some-msg".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &weak_auth_cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_200_and_adds_phone_number_to_user() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    {
        let req = AddPhoneNumberRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: AddPhoneNumberResponse = res.json().await;
        assert_eq!(res.resolution, AddPhoneNumberResolution::Added);
    }

    let cookie = {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let new_cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_ne!(new_cookie, cookie);

        new_cookie
    };

    {
        let res = fixture
            .client
            .get("/api/users/user")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: GetUserResponse = res.json().await;
        assert!(res.user.phone_numbers.contains(&"+11112223333".to_string()));
    }
}

#[tokio::test]
#[traced_test]
async fn users_phone_number_returns_200_after_only_signup_no_login() {
    let fixture = test_router_no_auth().await;

    let cookie = {
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

        let cookie = res
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(res.text().await, "Signup completed.");

        cookie
    };

    {
        let req = AddPhoneNumberRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: AddPhoneNumberResponse = res.json().await;
        assert_eq!(res.resolution, AddPhoneNumberResolution::Added);

        let sms = fixture.state.sms.as_ref() as &dyn Any;
        let sms = sms.downcast_ref::<TestSmsSender>().unwrap();

        let msgs = sms.all_msgs().await;
        assert_eq!(msgs.len(), 1);

        let msg = msgs.get(0).unwrap();
        assert_eq!(msg.to_phone, "+11112223333");
        assert_eq!(
            msg.content,
            format!(
                "[typesofants.org] your one-time code is: {}",
                first_sms_otp()
            )
        );
    }

    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert_ne!(new_cookie, cookie);
    }
}

#[tokio::test]
#[traced_test]
async fn users_phone_number_returns_200_and_sends_new_code() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    {
        let req = AddPhoneNumberRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: AddPhoneNumberResponse = res.json().await;
        assert_eq!(res.resolution, AddPhoneNumberResolution::Added);

        let sms = fixture.state.sms.as_ref() as &dyn Any;
        let sms = sms.downcast_ref::<TestSmsSender>().unwrap();

        let msgs = sms.all_msgs().await;
        assert_eq!(msgs.len(), 1);

        let msg = msgs.get(0).unwrap();
        assert_eq!(msg.to_phone, "+11112223333");
        assert_eq!(
            msg.content,
            format!(
                "[typesofants.org] your one-time code is: {}",
                first_sms_otp()
            )
        );
    }
}

#[tokio::test]
#[traced_test]
async fn users_phone_number_returns_200_and_cancels_previous_codes() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    // old one, sends first code
    let f = || async {
        let req = AddPhoneNumberRequest {
            phone_number: "+1 (111) 222-3333".to_string(),
            force_send: true,
        };

        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let res: AddPhoneNumberResponse = res.json().await;
        assert_eq!(res.resolution, AddPhoneNumberResolution::Added);
    };

    f().await; // send first code
    f().await; // send second, cancel first

    // The previous correct one now fails
    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: first_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // The actual correct one succeeds
    {
        let req = VerificationAttemptRequest {
            method: VerificationSubmission::Phone {
                phone_number: "+1 (111) 222-3333".to_string(),
                otp: second_sms_otp(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert_ne!(new_cookie, cookie);
    }
}
