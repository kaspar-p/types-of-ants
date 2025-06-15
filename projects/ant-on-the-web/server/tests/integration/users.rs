use crate::fixture::{test_router_auth, test_router_no_auth};
use http::{header::SET_COOKIE, StatusCode};
use tracing_test::traced_test;

use ant_on_the_web::{
    err::ValidationError,
    users::{
        EmailRequest, GetUserResponse, LoginMethod, LoginRequest, LoginResponse, SignupRequest,
    },
};

#[tokio::test]
async fn users_signup_returns_400_if_not_json() {
    let fixture = test_router_no_auth().await;

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
async fn users_signup_returns_400_if_username_invalid() {
    let fixture = test_router_no_auth().await;

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
        let j: ValidationError = res.json().await;
        let err = j.errors.first().unwrap();
        assert_eq!(err.field, "username");
        assert_eq!(err.msg, "Field must be between 3 and 16 characters.");
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
        let j: ValidationError = res.json().await;
        let err = j.errors.first().unwrap();
        assert_eq!(err.field, "username");
        assert_eq!(err.msg, "Field must be between 3 and 16 characters.");
    }

    {
        let req = SignupRequest {
            username: "-_*090][]".to_string(),
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
        let j: ValidationError = res.json().await;
        let err = j.errors.first().unwrap();
        assert_eq!(err.field, "username");
        assert_eq!(
            err.msg,
            "Field must contain only lowercase characters (a-z) and numbers (0-9)."
        );
    }
}

#[tokio::test]
async fn users_signup_returns_400_if_phone_number_invalid() {
    let fixture = test_router_no_auth().await;

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
    let j: ValidationError = res.json().await;
    let err = j.errors.first().unwrap();
    assert_eq!(err.field, "phoneNumber");
    assert_eq!(err.msg, "Field invalid.");
}

#[tokio::test]
async fn users_signup_returns_400_with_multiple_errors() {
    let fixture = test_router_no_auth().await;

    {
        let req = SignupRequest {
            username: "BAD__CHARACTERS__ERROR__AND__TOO_LONG".to_string(),
            email: "email@domain.com".to_string(),
            phone_number: "not a number".to_string(),
            password: "my-password".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/signup")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let j: ValidationError = res.json().await;

        assert_eq!(j.errors.len(), 4);

        let e_password = j.errors.iter().find(|f| f.field == "password").unwrap();
        assert_eq!(e_password.field, "password");
        assert_eq!(e_password.msg, "Field must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly.");

        let e_phone = j.errors.iter().find(|f| f.field == "phoneNumber").unwrap();
        assert_eq!(e_phone.field, "phoneNumber");
        assert_eq!(e_phone.msg, "Field invalid.");

        let e_username_len = j
            .errors
            .iter()
            .find(|f| f.field == "username" && f.msg.contains("between"))
            .unwrap();
        assert_eq!(e_username_len.field, "username");
        assert_eq!(
            e_username_len.msg,
            "Field must be between 3 and 16 characters."
        );

        let e_username_chars = j
            .errors
            .iter()
            .find(|f| f.field == "username" && f.msg.contains("contain"))
            .unwrap();
        assert_eq!(e_username_chars.field, "username");
        assert_eq!(
            e_username_chars.msg,
            "Field must contain only lowercase characters (a-z) and numbers (0-9)."
        );
    }
}

#[tokio::test]
async fn users_signup_returns_400_if_password_invalid() {
    let fixture = test_router_no_auth().await;

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
        let j: ValidationError = res.json().await;
        let err = j.errors.first().unwrap();

        assert_eq!(err.field, "password");
        assert_eq!(err.msg, "Field must contain the word 'ant'. Please do not reuse a password from another place, you are typing this into a website called typesofants.org, be a little silly.");
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
        let j: ValidationError = res.json().await;
        let err = j.errors.first().unwrap();
        assert_eq!(err.field, "password");
        assert_eq!(err.msg, "Field must be between 8 and 64 characters.");
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
        let j: ValidationError = res.json().await;
        let err = j.errors.first().unwrap();
        assert_eq!(err.field, "password");
        assert_eq!(err.msg, "Field must be between 8 and 64 characters.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_signup_returns_409_if_user_already_exists() {
    let fixture = test_router_no_auth().await;

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
async fn users_signup_succeeds() {
    let fixture = test_router_no_auth().await;

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

        let cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.contains("typesofants_auth="));
        assert!(cookie.contains("HttpOnly"));

        let text = res.text().await;
        assert_eq!(text, "Signup completed.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_signup_returns_409_if_user_already_signed_up() {
    let fixture = test_router_no_auth().await;

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

#[tokio::test]
#[traced_test]
async fn users_login_returns_401_if_no_corresponding_user() {
    let fixture = test_router_no_auth().await;

    // Username
    {
        let req = LoginRequest {
            method: LoginMethod::Username("someuser".to_string()),
            password: "somepassword".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }

    // Phone
    {
        let req = LoginRequest {
            method: LoginMethod::Phone("+2 (444) 222-3232".to_string()),
            password: "somepassword".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }

    // Email
    {
        let req = LoginRequest {
            method: LoginMethod::Email("some@email.ca".to_string()),
            password: "somepassword".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_logout_returns_4xx_if_not_authenticated() {
    let fixture = test_router_no_auth().await;

    {
        let res = fixture.client.post("/api/users/logout").send().await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(res.text().await, "Invalid authorization token.");
    }

    {
        let res = fixture
            .client
            .post("/api/users/logout")
            .header("Cookie", "typesofants_auth=abc")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_logout_returns_200_if_authenticated() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let res = fixture
            .client
            .post("/api/users/logout")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let expiration_cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(expiration_cookie.contains("typesofants_auth="));
        assert!(expiration_cookie.contains("HttpOnly"));
        assert!(expiration_cookie.contains("SameSite"));

        let text = res.text().await;
        assert_eq!(text, "Logout successful.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_login_returns_401_if_wrong_fields() {
    let fixture = test_router_no_auth().await;

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

    // Right password, wrong username.
    {
        let req = LoginRequest {
            method: LoginMethod::Username("username".to_string()),
            password: "my-ant-password".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }

    // Right password, wrong phone number.
    {
        let req = LoginRequest {
            method: LoginMethod::Phone("+2 (444) 111-2222".to_string()),
            password: "my-ant-password".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }

    // Right password, wrong email.
    {
        let req = LoginRequest {
            method: LoginMethod::Email("user@domain.ca".to_string()),
            password: "my-ant-password".to_string(),
        };

        let res = fixture
            .client
            .post("/api/users/login")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_login_returns_200_with_cookie_headers() {
    let fixture = test_router_no_auth().await;

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

    // Login includes Set-Cookie header with the right properties.
    {
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

        let cookie = res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.contains("typesofants_auth="));
        // assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("HttpOnly"));
    }
}

#[tokio::test]
#[traced_test]
async fn users_login_returns_200_returns_bearer_token() {
    let fixture = test_router_no_auth().await;

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

    // Login via username
    {
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
        assert!(res.access_token.contains(".")); // JWT standard mandates it
    }

    // Login via email
    {
        let req = LoginRequest {
            method: LoginMethod::Email("email@domain.com".to_string()),
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
        assert!(res.access_token.contains(".")); // JWT standard mandates it
    }

    // Login via phone
    {
        let req = LoginRequest {
            method: LoginMethod::Phone("+1 (111) 222-3333".to_string()),
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
        assert!(res.access_token.contains(".")); // JWT standard mandates it
    }
}

#[tokio::test]
#[traced_test]
async fn users_user_returns_401_if_token_has_been_tampered_with() {
    let fixture = test_router_no_auth().await;

    // Hit authenticated endpoint /users/user/{user_name}
    {
        let res = fixture
            .client
            .get("/api/users/user/nobody")
            .header("Cookie", "typesofants_auth=blahblahblah")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.text().await, "Access denied.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_user_returns_400_if_missing_token() {
    let fixture = test_router_no_auth().await;

    // No Cookie header at all
    {
        let res = fixture.client.get("/api/users/user/nobody").send().await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(res.text().await, "Invalid authorization token.");
    }

    // Not using the typesofants_auth name
    {
        let res = fixture
            .client
            .get("/api/users/user/nobody")
            .header("Cookie", "other_cookie=blahblahblah")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(res.text().await, "Invalid authorization token.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_user_returns_200_if_authn_token_right() {
    let (fixture, cookie) = test_router_auth().await;

    // Hit authenticated endpoint /users/user/{user_name}
    {
        let res = fixture
            .client
            .get("/api/users/user")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let res: GetUserResponse = res.json().await;
        assert_eq!(res.user.emails.len(), 1);
        assert_eq!(res.user.emails[0].as_str(), "email@domain.com");
        assert_eq!(res.user.username, "user");
        assert_ne!(res.user.password_hash, "my-ant-password");
    }
}

#[tokio::test]
#[traced_test]
async fn users_subscribe_newsletter_returns_400_if_malformed_email() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = EmailRequest {
            email: "blahblah".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/subscribe-newsletter")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let res: ValidationError = res.json().await;
        let v = res.errors.first().unwrap();
        assert_eq!(v.field, "email");
        assert_eq!(v.msg, "Field invalid.");
    }
}

#[tokio::test]
#[traced_test]
async fn users_subscribe_newsletter_returns_409_if_email_already_registered() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = EmailRequest {
            email: "brand-new@domain.com".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/subscribe-newsletter")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = EmailRequest {
            email: "brand-new@domain.com".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/subscribe-newsletter")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(res.text().await, "Already subscribed!");
    }
}

#[tokio::test]
#[traced_test]
async fn users_subscribe_newsletter_returns_200_for_unauthenticated_calls() {
    let (fixture, _) = test_router_auth().await;

    {
        let req = EmailRequest {
            email: "some-new-email@domain.com".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/subscribe-newsletter")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn users_subscribe_newsletter_returns_409_if_email_taken_by_another_user() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = EmailRequest {
            email: "some-new-email@domain.com".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/subscribe-newsletter")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = EmailRequest {
            email: "some-new-email@domain.com".to_string(),
        };
        let res = fixture
            .client
            .post("/api/users/subscribe-newsletter")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::CONFLICT);
    }
}
