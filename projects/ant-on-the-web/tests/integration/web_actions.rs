use ant_data_farm::web_actions::{WebAction, WebTargetType};
use ant_on_the_web::web_actions::WebActionRequest;
use http::{header::COOKIE, StatusCode};
use tracing::debug;
use tracing_test::traced_test;

use crate::fixture::{get_telemetry_cookie, test_router_auth, test_router_no_auth, FixtureOptions};

#[tokio::test]
#[traced_test]
async fn web_actions_action_returns_200_if_telemetry_missing_and_new_assigned() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    {
        let req = WebActionRequest {
            target_type: WebTargetType::Button,
            action: WebAction::Click,
            target: "some-button".to_string(),
        };
        let res = fixture
            .client
            .post("/api/web-actions/action")
            .json(&req)
            .send()
            .await;

        debug!("{:?}", res.headers());

        let telemetry = get_telemetry_cookie(res.headers());

        assert!(telemetry.contains("typesofants_telemetry="));
        assert!(telemetry.contains("HttpOnly"));
        assert!(telemetry.contains("Secure"));

        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn web_actions_action_returns_200_if_unauthenticated() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    {
        let req = WebActionRequest {
            target_type: WebTargetType::Button,
            action: WebAction::Click,
            target: "some-button".to_string(),
        };
        let res = fixture
            .client
            .post("/api/web-actions/action")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn web_actions_action_returns_200_if_authenticated() {
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

    {
        let req = WebActionRequest {
            target_type: WebTargetType::Button,
            action: WebAction::Click,
            target: "some-button".to_string(),
        };
        let res = fixture
            .client
            .post("/api/web-actions/action")
            .json(&req)
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn telemetry_cookie_stays_the_same_over_multiple_requests_and_keeps_auth() {
    let (fixture, cookie0) = test_router_auth(FixtureOptions::new()).await;

    let cookie1 = {
        let req = WebActionRequest {
            target_type: WebTargetType::Button,
            action: WebAction::Click,
            target: "some-button".to_string(),
        };
        let res = fixture
            .client
            .post("/api/web-actions/action")
            .json(&req)
            .header(COOKIE.as_str(), &cookie0)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        get_telemetry_cookie(res.headers())
    };

    let cookie2 = {
        let req = WebActionRequest {
            target_type: WebTargetType::Button,
            action: WebAction::Click,
            target: "some-button".to_string(),
        };
        let res = fixture
            .client
            .post("/api/web-actions/action")
            .json(&req)
            .header(COOKIE.as_str(), &cookie1)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        get_telemetry_cookie(res.headers())
    };

    assert_eq!(cookie1, cookie2);

    let cookie3 = {
        let req = WebActionRequest {
            target_type: WebTargetType::Button,
            action: WebAction::Click,
            target: "some-button".to_string(),
        };
        let res = fixture
            .client
            .post("/api/web-actions/action")
            .json(&req)
            .header(COOKIE.as_str(), &cookie2)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        get_telemetry_cookie(res.headers())
    };

    assert_eq!(cookie2, cookie3);
}
