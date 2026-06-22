use crate::{
    fixture::{FixtureOptions, TestFixture},
    fixture_sms::first_otp,
};
use ant_on_the_web::users::{
    AddPhoneNumberRequest, AddPhoneNumberResponse, AddResolution, LoginMethod, LoginRequest,
    SignupRequest, VerificationAttemptRequest, VerificationSubmission,
};
use http::StatusCode;
use tracing_test::traced_test;

/// Regression test for the phone-hijack vulnerability (issue #3107).
///
/// An attacker with their own weak-auth token calls `add_phone_number` on a phone
/// belonging to a victim. This cancels the victim's pending OTP and issues a new one
/// for the attacker's user_id. If the attacker submits that OTP, the server must reject
/// it because the user_id in the pending verification does not match the user_id in the
/// attacker's auth token.
///
/// Fix: `verification_attempt` must validate that the `user_id` returned by the
/// verification receipt matches the user_id in the auth token. Mismatches → 401.
#[tokio::test]
#[traced_test]
async fn regress_3253_verification_attempt_rejects_otp_when_user_id_mismatch() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let victim_phone = "+1 (555) 000-0001".to_string();

    // Victim signs up and adds their phone (triggers first OTP)

    let victim_cookie = {
        {
            let res = fixture
                .client
                .post("/api/users/signup")
                .json(&SignupRequest {
                    username: "victim".to_string(),
                    password: "my-ant-password".to_string(),
                    password2: "my-ant-password".to_string(),
                })
                .send()
                .await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        let token = {
            let res = fixture
                .client
                .post("/api/users/login")
                .json(&LoginRequest {
                    method: LoginMethod::Username("victim".to_string()),
                    password: "my-ant-password".to_string(),
                })
                .send()
                .await;
            assert_eq!(res.status(), StatusCode::OK);
            res.json::<ant_on_the_web::users::LoginResponse>()
                .await
                .access_token
        };

        format!("typesofants_auth={token}")
    };

    // Victim requests an OTP be sent to their phone (first_otp() is consumed here).
    {
        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &victim_cookie)
            .json(&AddPhoneNumberRequest {
                phone_number: victim_phone.clone(),
                force_send: false,
            })
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.json::<AddPhoneNumberResponse>().await.resolution,
            AddResolution::Added
        );
    }

    // Attacker signs up and targets the same phone

    let attacker_cookie = {
        {
            let res = fixture
                .client
                .post("/api/users/signup")
                .json(&SignupRequest {
                    username: "attacker".to_string(),
                    password: "my-ant-password".to_string(),
                    password2: "my-ant-password".to_string(),
                })
                .send()
                .await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        let token = {
            let res = fixture
                .client
                .post("/api/users/login")
                .json(&LoginRequest {
                    method: LoginMethod::Username("attacker".to_string()),
                    password: "my-ant-password".to_string(),
                })
                .send()
                .await;
            assert_eq!(res.status(), StatusCode::OK);
            res.json::<ant_on_the_web::users::LoginResponse>()
                .await
                .access_token
        };

        format!("typesofants_auth={token}")
    };

    // Attacker submits first_otp() (the OTP sent to the victim's phone) using the
    // ATTACKER'S own weak-auth cookie. The verification record's user_id is the attacker's,
    // but it was triggered by targeting the victim's phone — the server must reject this
    // because the pending verification's user_id does not match the auth token's user_id.
    {
        let res = fixture
            .client
            .post("/api/users/verification-attempt")
            .header("Cookie", &attacker_cookie)
            .json(&VerificationAttemptRequest {
                method: VerificationSubmission::Phone {
                    phone_number: victim_phone.clone(),
                    otp: first_otp(),
                },
            })
            .send()
            .await;
        assert_eq!(
            res.status(),
            StatusCode::BAD_REQUEST,
            "server must reject verification when the OTP's user_id does not match the auth token"
        );
    }

    // Victim can still add their own phone — the attacker's failed attempt must not
    // permanently block the victim's number.
    {
        let res = fixture
            .client
            .post("/api/users/phone-number")
            .header("Cookie", &victim_cookie)
            .json(&AddPhoneNumberRequest {
                phone_number: victim_phone.clone(),
                force_send: false,
            })
            .send()
            .await;
        assert_eq!(
            res.status(),
            StatusCode::OK,
            "victim must still be able to add their own phone after the attack is rejected"
        );
    }
}
