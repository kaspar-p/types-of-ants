use std::any::Any;

use crate::fixture::{test_router_auth, test_router_no_auth, test_router_weak_auth, TestSmsSender};
use ant_on_the_web::{
    two_factor::VerificationMethod,
    users::{
        SignupRequest, VerificationAttemptRequest, VerificationRequest, VerificationSubmission,
    },
};
use http::{header::SET_COOKIE, StatusCode};
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn users_signup_returns_200_and_sends_one_time_codes() {
    let fixture = test_router_no_auth().await;

    {
        let req = SignupRequest {
            username: "user1".to_string(),
            email: "email1@domain.com".to_string(),
            phone_number: "+1 (212) 323-5445".to_string(),
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

    let sms = fixture.state.sms.as_ref() as &dyn Any;
    let sms = sms.downcast_ref::<TestSmsSender>().unwrap();

    let msgs = sms.all_msgs().await;
    assert_eq!(msgs.len(), 1);

    let msg = msgs.first().unwrap();
    assert_eq!(msg.to_phone, "+12123235445");
    assert_eq!(
        msg.content,
        "[typesofants.org] your one-time code is: ANT-qg7i2"
    );
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_401_if_unauthenticated_call() {
    let fixture = test_router_no_auth().await;

    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: "ANT-here".to_string(),
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

    // based on the deterministic testing rng
    let otp = "ANT-qg7i2";

    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

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
    let (fixture, cookie) = test_router_auth().await;

    // based on the deterministic testing rng
    let otp = "ANT-qg7i2";

    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(new_cookie.contains("typesofants_auth="));
        assert!(new_cookie.contains("HttpOnly"));

        // They must be different!
        assert_ne!(new_cookie, cookie);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_attempt_returns_400_for_wrong_or_too_many_attempts() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    for _ in 0..10 {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: "ANT-wrong".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // based on the deterministic testing rng
    let correct_otp = "ANT-qg7i2";

    // Even the correct one fails after too many bad attempts
    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: correct_otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", cookie.as_str())
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

    let correct_otp = "ANT-qg7i2"; // based on the deterministic testing rng
    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: correct_otp.to_string(),
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

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert_ne!(new_cookie, cookie);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_returns_200_after_only_signup_no_login() {
    let fixture = test_router_no_auth().await;

    let cookie = {
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
        let req = VerificationRequest {
            method: VerificationMethod::Phone("+1 (111) 222-3333".to_string()),
        };

        let res = fixture
            .client
            .post("/api/users/verification")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let sms = fixture.state.sms.as_ref() as &dyn Any;
        let sms = sms.downcast_ref::<TestSmsSender>().unwrap();

        let msgs = sms.all_msgs().await;
        assert_eq!(msgs.len(), 2); // one for the original signup, one for the /verification request

        let msg = msgs.get(0).unwrap();
        assert_eq!(msg.to_phone, "+11112223333");
        assert_eq!(
            msg.content,
            "[typesofants.org] your one-time code is: ANT-qg7i2"
        );

        let msg = msgs.get(1).unwrap();
        assert_eq!(msg.to_phone, "+11112223333");
        assert_eq!(
            msg.content,
            "[typesofants.org] your one-time code is: ANT-g5qp3"
        );
    }

    let correct_otp = "ANT-g5qp3"; // based on the deterministic testing rng
    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: correct_otp.to_string(),
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

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert_ne!(new_cookie, cookie);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_returns_200_and_sends_new_code() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    {
        let req = VerificationRequest {
            method: VerificationMethod::Phone("+1 (111) 222-3333".to_string()),
        };

        let res = fixture
            .client
            .post("/api/users/verification")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let sms = fixture.state.sms.as_ref() as &dyn Any;
        let sms = sms.downcast_ref::<TestSmsSender>().unwrap();

        let msgs = sms.all_msgs().await;
        assert_eq!(msgs.len(), 2); // one for the original signup, one for the /verification request

        let msg = msgs.get(0).unwrap();
        assert_eq!(msg.to_phone, "+11112223333");
        assert_eq!(
            msg.content,
            "[typesofants.org] your one-time code is: ANT-qg7i2"
        );

        let msg = msgs.get(1).unwrap();
        assert_eq!(msg.to_phone, "+11112223333");
        assert_eq!(
            msg.content,
            "[typesofants.org] your one-time code is: ANT-g5qp3"
        );
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_returns_200_and_cancels_previous_codes() {
    let (fixture, cookie) = test_router_weak_auth(None).await;

    {
        let req = VerificationRequest {
            method: VerificationMethod::Phone("+1 (111) 222-3333".to_string()),
        };

        let res = fixture
            .client
            .post("/api/users/verification")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    let previous_correct_otp = "ANT-qg7i2"; // based on the deterministic testing rng

    // The previous correct one now fails
    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: previous_correct_otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    let actual_correct_otp = "ANT-g5qp3"; // based on the deterministic testing rng

    // The actual correct one succeeds
    {
        let req = VerificationAttemptRequest {
            submission: VerificationSubmission::Phone {
                otp: actual_correct_otp.to_string(),
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

        let new_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert_ne!(new_cookie, cookie);
    }
}
