use crate::{
    middleware,
    types::{DbRouter, DbState},
};
use ant_data_farm::{
    ants::{Ant, AntStatus},
    users::UserId,
    DaoTrait,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

const PAGE_SIZE: usize = 1_000_usize;

async fn all_ants(State(dao): DbState) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(dao.ants.read().await.get_all().await).into_response(),
    )
}

#[derive(Serialize, Deserialize)]
struct Pagination {
    page: usize,
}
async fn unreleased_ants(State(dao): DbState, query: Query<Pagination>) -> impl IntoResponse {
    let ants = dao.ants.read().await;

    let mut unreleased_ants = ants
        .get_all()
        .await
        .iter()
        .filter_map(|&ant| match ant.status {
            AntStatus::Unreleased => Some(ant),
            _ => None,
        })
        .collect::<Vec<&Ant>>();
    unreleased_ants.sort();
    unreleased_ants.reverse();

    let ants_page = unreleased_ants.chunks(PAGE_SIZE).nth(query.page);
    match ants_page {
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(format!("No page {} exists!", query.page)).into_response(),
            )
        }
        Some(unreleased_ants) => {
            return (StatusCode::OK, Json(unreleased_ants).into_response());
        }
    }
}

async fn declined_ants(State(dao): DbState, query: Query<Pagination>) -> impl IntoResponse {
    let ants = dao.ants.read().await;

    let declined_ants = ants
        .get_all()
        .await
        .iter()
        .filter_map(|&ant| match ant.status {
            AntStatus::Released(_) => Some(ant),
            _ => None,
        })
        .collect::<Vec<&Ant>>();

    let ants_page = declined_ants.chunks(PAGE_SIZE).nth(query.page);
    match ants_page {
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(format!("No page {} exists!", query.page)).into_response(),
            )
        }
        Some(released_ants) => {
            return (StatusCode::OK, Json(released_ants).into_response());
        }
    }
}

async fn released_ants(State(dao): DbState, query: Query<Pagination>) -> impl IntoResponse {
    let ants = dao.ants.read().await;

    let released_ants = ants
        .get_all()
        .await
        .iter()
        .filter_map(|&ant| match ant.status {
            AntStatus::Released(_) => Some(ant),
            _ => None,
        })
        .collect::<Vec<&Ant>>();

    let ants_page = released_ants.chunks(PAGE_SIZE).nth(query.page);
    match ants_page {
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(format!("No page {} exists!", query.page)).into_response(),
            )
        }
        Some(released_ants) => {
            return (StatusCode::OK, Json(released_ants).into_response());
        }
    }
}

async fn latest_release(State(dao): DbState) -> impl IntoResponse {
    let release = dao.releases.read().await.get_latest_release().await;
    (StatusCode::OK, Json(release))
}

#[derive(Serialize, Deserialize)]
struct LatestAntsResponse {
    #[serde(with = "chrono::serde::ts_seconds")]
    date: chrono::DateTime<chrono::Utc>,
    release: i32,
    ants: Vec<Ant>,
}
async fn latest_ants(State(dao): DbState) -> impl IntoResponse {
    let ants = dao.ants.read().await;
    let releases = dao.releases.read().await;

    let all_ants: Vec<Ant> = ants.get_all().await.iter().map(|&x| x.clone()).collect();
    let latest_release = releases.get_latest_release().await;
    let current_release_ants = all_ants
        .iter()
        .filter(|ant| match ant.status {
            AntStatus::Released(n) => n == latest_release,
            _ => false,
        })
        .map(std::clone::Clone::clone)
        .collect::<Vec<Ant>>();

    (
        StatusCode::OK,
        Json(LatestAntsResponse {
            date: chrono::offset::Utc::now(),
            release: latest_release,
            ants: current_release_ants,
        }),
    )
}

#[derive(Deserialize, Debug)]
struct Suggestion {
    pub user_id: Option<String>,
    pub suggestion_content: String,
}

async fn make_suggestion(
    State(dao): DbState,
    Json(suggestion): Json<Suggestion>,
) -> impl IntoResponse {
    debug!("Top of /api/ant/suggest");
    let users = dao.users.read().await;
    let mut ants = dao.ants.write().await;

    let o_user = match &suggestion.user_id {
        None => users.get_one_by_name("nobody").await,
        Some(u) => {
            users
                .get_one_by_id(&UserId(Uuid::parse_str(u).unwrap()))
                .await
        }
    };

    if o_user.is_none() {
        if suggestion.user_id.is_some() {
            return (
                StatusCode::NOT_FOUND,
                Json("NOT_FOUND".to_string()).into_response(),
            );
        }
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("Unable to process ant suggestion!").into_response(),
        );
    }

    let user = o_user.unwrap();
    let res = ants
        .add_unreleased_ant(suggestion.suggestion_content, user.user_id)
        .await;
    if res.is_err() {
        debug!("Encountered error: {}", res.unwrap_err());
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("Unable to process ant suggestion!").into_response(),
        );
    }

    (
        StatusCode::OK,
        Json("Added suggestion, thanks!").into_response(),
    )
}

// #[derive(Serialize, Deserialize)]
// struct TweetData {
//     pub ant_id: Uuid,
// }
// async fn tweet(State(dao): DaoState, Json(tweet_data): Json<TweetData>) -> impl IntoResponse {
//     let mut ants = dao.ants.write().await;
//     let ant_id = AntId(tweet_data.ant_id);
//     match ants.get_one_by_id(&ant_id).await {
//         None => {
//             return (
//                 StatusCode::NOT_FOUND,
//                 Json(format!("The ant with ID '{}' didn't exist!", ant_id)).into_response(),
//             )
//         }
//         Some(_) => (),
//     }

//     return match ants.add_ant_tweet(ant_id).await {
//         None => (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json(format!("Failed to tweet ant with ID '{}'", ant_id)).into_response(),
//         ),
//         Some(ant) => (StatusCode::OK, Json(ant).into_response()),
//     };
// }

pub fn router() -> DbRouter {
    Router::new()
        .route_with_tsr("/latest-ants", get(latest_ants))
        .route_with_tsr("/unreleased-ants", get(unreleased_ants))
        .route_with_tsr("/released-ants", get(released_ants))
        .route_with_tsr("/declined-ants", get(declined_ants))
        .route_with_tsr("/all-ants", get(all_ants))
        .route_with_tsr("/latest-release", get(latest_release))
        .route_with_tsr("/suggest", post(make_suggestion))
        // .route_with_tsr("/tweet", post(tweet))
        .fallback(|| async {
            middleware::fallback(&[
                "GET /latest-ants",
                "GET /unreleased-ants",
                "GET /released-ants",
                "GET /declined-ants",
                "GET /all-ants",
                "GET /latest-release",
                "POST /suggest",
                // "POST /tweet",
            ])
        })
}
