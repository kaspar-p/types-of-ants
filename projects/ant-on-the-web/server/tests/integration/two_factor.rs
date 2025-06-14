use std::any::Any;

use crate::fixture::{
    authn_no_verify_test_router, authn_test_router, no_auth_test_router, TestSmsSender,
};
use ant_on_the_web::users::{SignupRequest, VerificationRequest, VerificationSubmission};
use http::{header::SET_COOKIE, StatusCode};
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn users_signup_returns_200_and_sends_one_time_codes() {
    let fixture = no_auth_test_router().await;

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
async fn users_verification_returns_401_if_unauthenticated_call() {
    let fixture = no_auth_test_router().await;

    {
        let req = VerificationRequest {
            submission: VerificationSubmission::Phone {
                otp: "ANT-here".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification")
            .header("Cookie", "typesofants_auth=other")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn users_verification_returns_200_with_different_cookie_headers() {
    let (fixture, cookie) = authn_no_verify_test_router().await;

    // based on the deterministic testing rng
    let otp = "ANT-qg7i2";

    {
        let req = VerificationRequest {
            submission: VerificationSubmission::Phone {
                otp: otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification")
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
async fn users_verification_returns_200_with_different_cookie_headers_even_if_already_authn() {
    let (fixture, cookie) = authn_test_router().await;

    // based on the deterministic testing rng
    let otp = "ANT-qg7i2";

    {
        let req = VerificationRequest {
            submission: VerificationSubmission::Phone {
                otp: otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification")
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
async fn users_verification_returns_400_for_wrong_or_too_many_attempts() {
    let (fixture, cookie) = authn_no_verify_test_router().await;

    for _ in 0..10 {
        let req = VerificationRequest {
            submission: VerificationSubmission::Phone {
                otp: "ANT-wrong".to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification")
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
        let req = VerificationRequest {
            submission: VerificationSubmission::Phone {
                otp: correct_otp.to_string(),
            },
        };

        let res = fixture
            .client
            .post("/api/users/verification")
            .header("Cookie", cookie.as_str())
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
