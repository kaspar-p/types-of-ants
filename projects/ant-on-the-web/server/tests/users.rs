use fixture::test_router;
use http::StatusCode;
use tracing_test::traced_test;

use ant_on_the_web::users::{LoginMethod, LoginRequest, LoginResponse, SignupRequest};

mod fixture;

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

#[tokio::test]
#[traced_test]
async fn user_login_with_no_corresponding_user_gets_unauthorized() {
    let fixture = test_router().await;

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
async fn user_signup_and_bad_login_returns_unauthorized() {
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
async fn user_login_after_signup_returns_token() {
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
        assert!(res.token.contains(".")); // JWT standard mandates it
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
        assert!(res.token.contains(".")); // JWT standard mandates it
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
        assert!(res.token.contains(".")); // JWT standard mandates it
    }
}
