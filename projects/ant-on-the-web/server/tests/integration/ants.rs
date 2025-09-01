use crate::fixture::{test_router_auth, test_router_no_auth};
use ant_data_farm::ants::AntId;
use ant_on_the_web::{
    ants::{
        FavoriteAntRequest, FavoriteAntResponse, LatestAntsResponse, ReleasedAntsResponse,
        SuggestionRequest, TotalResponse, UnfavoriteAntRequest,
    },
    err::ValidationError,
};
use http::StatusCode;
use tracing_test::traced_test;
use uuid::Uuid;

#[tokio::test]
#[traced_test]
async fn ants_total_matches_ants_released() {
    let fixture = test_router_no_auth().await;

    let ants_res = fixture
        .client
        .get("/api/ants/released-ants?page=0")
        .send()
        .await;
    assert_eq!(ants_res.status(), StatusCode::OK);
    let ants: ReleasedAntsResponse = ants_res.json().await;

    let total_res = fixture.client.get("/api/ants/total").send().await;
    assert_eq!(total_res.status(), StatusCode::OK);
    let total: TotalResponse = total_res.json().await;

    assert!(ants.ants.len() <= 1000); // page size
    assert!(ants.ants.len() <= total.total);

    let mut running_total = ants.ants.len();
    let mut has_next_page = true;
    let mut next_page = 1;
    while has_next_page {
        let ants_res = fixture
            .client
            .get(format!("/api/ants/released-ants?page={next_page}").as_str())
            .send()
            .await;
        assert_eq!(ants_res.status(), StatusCode::OK);
        let ants: ReleasedAntsResponse = ants_res.json().await;

        running_total += ants.ants.len();
        if ants.has_next_page {
            next_page += 1;
        } else {
            has_next_page = false;
        }
    }

    assert_eq!(running_total, total.total);
}

#[tokio::test]
#[traced_test]
async fn ants_suggest_returns_200_with_user_if_authenticated() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = SuggestionRequest {
            suggestion_content: "some ant content".to_string(),
        };
        let res = fixture
            .client
            .post("/api/ants/suggest")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_suggest_returns_200_even_if_not_authenticated() {
    let fixture = test_router_no_auth().await;

    let req = SuggestionRequest {
        suggestion_content: "some ant content".to_string(),
    };
    let res = fixture
        .client
        .post("/api/ants/suggest")
        .json(&req)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
#[traced_test]
async fn ants_favorite_returns_401_if_not_authenticated() {
    let fixture = test_router_no_auth().await;

    {
        let req = FavoriteAntRequest {
            ant_id: AntId(Uuid::new_v4()),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = FavoriteAntRequest {
            ant_id: AntId(Uuid::new_v4()),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .json(&req)
            .header("Cookie", "typesofants_auth=something-bad")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_favorite_returns_400_if_no_such_ant() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = FavoriteAntRequest {
            ant_id: AntId(Uuid::nil()),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body: ValidationError = res.json().await;

        assert_eq!(body.errors.len(), 1);
        assert_eq!(body.errors.first().unwrap().field, "antId");
        assert_eq!(body.errors.first().unwrap().msg, "No such ant.");
    }
}

#[tokio::test]
#[traced_test]
async fn ants_favorite_returns_200_idempotently() {
    let (fixture, cookie) = test_router_auth().await;

    let ant_id = {
        let res = fixture
            .client
            .get("/api/ants/latest-ants")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: LatestAntsResponse = res.json().await;

        body.ants.first().unwrap().ant_id
    };

    let t1 = {
        let req = FavoriteAntRequest {
            ant_id: ant_id.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: FavoriteAntResponse = res.json().await;

        body.favorited_at
    };

    let t2 = {
        let req = FavoriteAntRequest {
            ant_id: ant_id.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: FavoriteAntResponse = res.json().await;

        body.favorited_at
    };

    assert_eq!(t1, t2);
}

#[tokio::test]
#[traced_test]
async fn ants_unfavorite_returns_401_if_not_authenticated() {
    let fixture = test_router_no_auth().await;

    {
        let req = FavoriteAntRequest {
            ant_id: AntId(Uuid::new_v4()),
        };
        let res = fixture
            .client
            .post("/api/ants/unfavorite")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = FavoriteAntRequest {
            ant_id: AntId(Uuid::new_v4()),
        };
        let res = fixture
            .client
            .post("/api/ants/unfavorite")
            .json(&req)
            .header("Cookie", "typesofants_auth=something-bad")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_unfavorite_returns_400_if_no_such_ant() {
    let (fixture, cookie) = test_router_auth().await;

    {
        let req = FavoriteAntRequest {
            ant_id: AntId(Uuid::nil()),
        };
        let res = fixture
            .client
            .post("/api/ants/unfavorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body: ValidationError = res.json().await;

        assert_eq!(body.errors.len(), 1);
        assert_eq!(body.errors.first().unwrap().field, "antId");
        assert_eq!(body.errors.first().unwrap().msg, "No such ant.");
    }
}

#[tokio::test]
#[traced_test]
async fn ants_unfavorite_returns_200_idempotently_and_unfavorites() {
    let (fixture, cookie) = test_router_auth().await;

    let ant_id = {
        let res = fixture
            .client
            .get("/api/ants/latest-ants")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: LatestAntsResponse = res.json().await;

        body.ants.first().unwrap().ant_id
    };

    let t1 = {
        let req = FavoriteAntRequest {
            ant_id: ant_id.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: FavoriteAntResponse = res.json().await;

        body.favorited_at
    };

    {
        let req = UnfavoriteAntRequest {
            ant_id: ant_id.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/unfavorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    };

    let t2 = {
        let req = FavoriteAntRequest {
            ant_id: ant_id.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/favorite")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: FavoriteAntResponse = res.json().await;

        body.favorited_at
    };

    assert_ne!(t1, t2);
    assert!(t2 > t1);
}
