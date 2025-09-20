use crate::fixture::{
    test_router_admin_auth, test_router_auth, test_router_no_auth, FixtureOptions,
};
use ant_data_farm::{ants::AntId, releases::AntReleaseRequest};
use ant_on_the_web::{
    ants::{
        CreateReleaseRequest, CreateReleaseResponse, DeclineAntRequest, DeclinedAntsRequest,
        DeclinedAntsResponse, FavoriteAntRequest, FavoriteAntResponse, GetReleaseRequest,
        GetReleaseResponse, LatestAntsResponse, LatestReleaseResponse, ReleasedAntsResponse,
        SuggestionRequest, SuggestionResponse, TotalResponse, UnfavoriteAntRequest,
        UnreleasedAntsResponse,
    },
    err::ValidationError,
};
use http::StatusCode;
use tracing_test::traced_test;
use uuid::Uuid;

#[tokio::test]
#[traced_test]
async fn ants_total_matches_ants_released() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

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
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

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
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

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
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

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
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

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
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

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
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

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
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

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
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

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

#[tokio::test]
#[traced_test]
async fn ants_release_post_returns_401_if_not_admin() {
    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_release_post_returns_200_if_release_made() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    let ants = {
        let res1 = fixture
            .client
            .post("/api/ants/suggest")
            .header("Cookie", &cookie)
            .json(&SuggestionRequest {
                suggestion_content: "ant suggestion!".to_string(),
            })
            .send()
            .await;

        assert_eq!(res1.status(), StatusCode::OK);
        let body1: SuggestionResponse = res1.json().await;

        let res2 = fixture
            .client
            .post("/api/ants/suggest")
            .header("Cookie", &cookie)
            .json(&SuggestionRequest {
                suggestion_content: "other ant suggestion!".to_string(),
            })
            .send()
            .await;

        assert_eq!(res2.status(), StatusCode::OK);
        let body2: SuggestionResponse = res2.json().await;

        &[body1.ant, body2.ant]
    };

    let latest1 = {
        let res = fixture.client.get("/api/ants/latest-ants").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: LatestAntsResponse = res.json().await;

        body
    };

    let release = {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![
                AntReleaseRequest {
                    ant_id: ants[0].ant_id,
                    overwrite_content: None,
                },
                AntReleaseRequest {
                    ant_id: ants[1].ant_id,
                    overwrite_content: Some("something else, didn't like that one".to_string()),
                },
            ],
        };

        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: CreateReleaseResponse = res.json().await;

        body.release
    };

    let latest2 = {
        let res = fixture.client.get("/api/ants/latest-ants").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: LatestAntsResponse = res.json().await;

        body
    };

    assert_ne!(latest1, latest2);
    assert_eq!(latest1.release + 1, latest2.release);
    assert_eq!(latest2.release, release);
}

#[tokio::test]
#[traced_test]
async fn ants_release_post_returns_400_if_validation_error() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    // prereq: suggest 1000 things
    let mut ids: Vec<AntId> = vec![];
    {
        for i in 0..1000 {
            let res = fixture
                .client
                .post("/api/ants/suggest")
                .header("Cookie", &cookie)
                .json(&SuggestionRequest {
                    suggestion_content: format!("ant suggestion {}", i),
                })
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);
            let body: SuggestionResponse = res.json().await;

            ids.push(body.ant.ant_id);
        }
    }

    // 400 if no ants included
    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "ants");
        assert_eq!(body.errors.first().unwrap().msg, "Ants cannot be empty.");
    }

    // 400 if too many ants included
    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: ids
                .iter()
                .map(|id| AntReleaseRequest {
                    ant_id: *id,
                    overwrite_content: None,
                })
                .collect(),
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "ants");
        assert_eq!(
            body.errors.first().unwrap().msg,
            "Ants too long, cannot exceed 256."
        );
    }

    // 400 if duplicate ants included
    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![
                AntReleaseRequest {
                    ant_id: ids[0],
                    overwrite_content: None,
                },
                AntReleaseRequest {
                    ant_id: ids[0],
                    overwrite_content: None,
                },
            ],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "ants");
        assert_eq!(
            body.errors.first().unwrap().msg,
            format!("Ant {} suggested more than once.", ids[0])
        );
    }

    // 400 if some ants are already released
    {
        let released_ant = {
            let res = fixture.client.get("/api/ants/latest-ants").send().await;

            assert_eq!(res.status(), StatusCode::OK);
            let body: LatestAntsResponse = res.json().await;

            body.ants[0].clone()
        };

        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![AntReleaseRequest {
                ant_id: released_ant.ant_id.clone(),
                overwrite_content: None,
            }],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "ants");
        assert_eq!(
            body.errors.first().unwrap().msg,
            format!(
                "Only unreleased ants may be suggested, ant {} is {}",
                released_ant.ant_id, released_ant.status
            )
        );
    }

    // 400 if overwrite_content too short
    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![AntReleaseRequest {
                ant_id: ids[0],
                overwrite_content: Some("".to_string()),
            }],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "content");
        assert_eq!(
            body.errors.first().unwrap().msg,
            "Ant content must be between 3 and 100 characters."
        );
    }

    // 400 if overwrite_content too long
    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![AntReleaseRequest {
                ant_id: ids[0],
                overwrite_content: Some("text".repeat(100).to_string()),
            }],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "content");
        assert_eq!(
            body.errors.first().unwrap().msg,
            "Ant content must be between 3 and 100 characters."
        );
    }

    // 400 if some ant didn't exist
    {
        let req = CreateReleaseRequest {
            label: "release".to_string(),
            ants: vec![AntReleaseRequest {
                ant_id: AntId(Uuid::nil()),
                overwrite_content: None,
            }],
        };
        let res = fixture
            .client
            .post("/api/ants/release")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.first().unwrap().field, "ants");
        assert_eq!(
            body.errors.first().unwrap().msg,
            format!("No such ant: {}", Uuid::nil()),
        );
    }
}

#[tokio::test]
#[traced_test]
async fn ants_release_get_returns_200_same_as_latest_release() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    let latest_release = {
        let res = fixture.client.get("/api/ants/latest-release").send().await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: LatestReleaseResponse = res.json().await;
        body
    };

    {
        let req = GetReleaseRequest {
            release: latest_release.release.release_number,
        };
        let res = fixture
            .client
            .get("/api/ants/release")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: GetReleaseResponse = res.json().await;

        assert_eq!(latest_release.release, body.release);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_release_get_returns_404_if_no_such_release() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    {
        let req = GetReleaseRequest { release: 99999 };
        let res = fixture
            .client
            .get("/api/ants/release")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_decline_returns_4xx_if_not_admin() {
    let fixture = test_router_no_auth(FixtureOptions::new()).await;

    {
        let req = DeclineAntRequest {
            ant_id: AntId(Uuid::nil()),
        };
        let res = fixture
            .client
            .post("/api/ants/decline")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    {
        let req = DeclineAntRequest {
            ant_id: AntId(Uuid::nil()),
        };
        let res = fixture
            .client
            .post("/api/ants/decline")
            .header("Cookie", "typesofants_auth=bad")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    let (fixture, cookie) = test_router_auth(FixtureOptions::new()).await;

    {
        let req = DeclineAntRequest {
            ant_id: AntId(Uuid::nil()),
        };
        let res = fixture
            .client
            .post("/api/ants/decline")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
#[traced_test]
async fn ants_decline_returns_400_if_ant_not_exists() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    {
        let req = DeclineAntRequest {
            ant_id: AntId(Uuid::nil()),
        };
        let res = fixture
            .client
            .post("/api/ants/decline")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.len(), 1);
        assert_eq!(body.errors[0].field, "ant_id");
        assert_eq!(body.errors[0].msg, "No such ant.");
    }
}

#[tokio::test]
#[traced_test]
async fn ants_decline_returns_400_if_ant_already_declined_or_released() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    {
        let declined_ant = {
            let res = fixture
                .client
                .get("/api/ants/declined-ants")
                .header("Cookie", &cookie)
                .json(&DeclinedAntsRequest { page: 0 })
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);
            let body: DeclinedAntsResponse = res.json().await;

            body.ants.last().unwrap().ant_id
        };

        {
            let req = DeclineAntRequest {
                ant_id: declined_ant.clone(),
            };
            let res = fixture
                .client
                .post("/api/ants/decline")
                .header("Cookie", &cookie)
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::BAD_REQUEST);

            let body: ValidationError = res.json().await;
            assert_eq!(body.errors.len(), 1);
            assert_eq!(body.errors[0].field, "ant_id");
            assert_eq!(body.errors[0].msg, "Ant already declined.");
        }
    }

    {
        let released_ant = {
            let res = fixture
                .client
                .get("/api/ants/released-ants?page=0")
                .header("Cookie", &cookie)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);
            let body: ReleasedAntsResponse = res.json().await;

            body.ants.first().unwrap().ant_id
        };

        {
            let req = DeclineAntRequest {
                ant_id: released_ant.clone(),
            };
            let res = fixture
                .client
                .post("/api/ants/decline")
                .header("Cookie", &cookie)
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::BAD_REQUEST);

            let body: ValidationError = res.json().await;
            assert_eq!(body.errors.len(), 1);
            assert_eq!(body.errors[0].field, "ant_id");
            assert_eq!(body.errors[0].msg, "Ant already released.");
        }
    }
}

#[tokio::test]
#[traced_test]
async fn ants_decline_returns_200_for_declining_new_ant() {
    let (fixture, cookie) = test_router_admin_auth(FixtureOptions::new()).await;

    let unreleased_ant = {
        let res = fixture
            .client
            .get("/api/ants/unreleased-ants?page=0")
            .header("Cookie", &cookie)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: UnreleasedAntsResponse = res.json().await;

        body.ants.first().unwrap().ant_id
    };

    {
        let req = DeclineAntRequest {
            ant_id: unreleased_ant.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/decline")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    {
        let req = DeclineAntRequest {
            ant_id: unreleased_ant.clone(),
        };
        let res = fixture
            .client
            .post("/api/ants/decline")
            .header("Cookie", &cookie)
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body: ValidationError = res.json().await;
        assert_eq!(body.errors.len(), 1);
        assert_eq!(body.errors[0].field, "ant_id");
        assert_eq!(body.errors[0].msg, "Ant already declined.");
    }
}
