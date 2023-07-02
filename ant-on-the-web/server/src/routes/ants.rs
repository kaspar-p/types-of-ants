use crate::{
    middleware::fallback,
    types::{DaoRouter, DaoState},
};
use ant_data_farm::{
    ants::{Ant, AntId},
    users::UserId,
    DaoTrait,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct AllAntsResponse {
    ants: Vec<Ant>,
}
async fn all_ants(State(dao): DaoState) -> impl IntoResponse {
    let ants = dao.ants.read().await;
    let all_ants = ants.get_all().await.iter().map(|&x| x.clone()).collect();
    (StatusCode::OK, Json(AllAntsResponse { ants: all_ants }))
}

#[derive(Serialize, Deserialize)]
struct LatestAntsResponse {
    #[serde(with = "chrono::serde::ts_seconds")]
    date: chrono::DateTime<chrono::Utc>,
    release: i32,
    ants: Vec<Ant>,
}

async fn latest_release(State(dao): DaoState) -> impl IntoResponse {
    let release = dao.releases.read().await.get_latest_release().await;
    (StatusCode::OK, Json(release))
}

async fn latest_ants(State(dao): DaoState) -> impl IntoResponse {
    let ants = dao.ants.read().await;
    let releases = dao.releases.read().await;

    let all_ants: Vec<Ant> = ants.get_all().await.iter().map(|&x| x.clone()).collect();
    let latest_release = releases.get_latest_release().await;
    let current_release_ants = all_ants
        .iter()
        .filter(|ant| ant.released == latest_release)
        .map(|ant| ant.to_owned())
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
// #[axum_macros::debug_handler]
async fn make_suggestion(
    State(dao): DaoState,
    Json(suggestion): Json<Suggestion>,
) -> impl IntoResponse {
    debug!("Top of /api/ant/suggest");
    let users = dao.users.read().await;
    let mut ants = dao.ants.write().await;

    let o_user = match &suggestion.user_id {
        None => users.get_one_by_name(&String::from("nobody")).await,
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
                Json(format!(
                    "NOT_FOUND" // "There was no user with ID: {:?}",
                                // suggestion.user_id
                ))
                .into_response(),
            );
        } else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Unable to process ant suggestion!").into_response(),
            );
        }
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

    return (
        StatusCode::OK,
        Json("Added suggestion, thanks!").into_response(),
    );
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

pub fn router() -> DaoRouter {
    Router::new()
        .route_with_tsr("/suggest", post(make_suggestion))
        // .route_with_tsr("/tweet", post(tweet))
        .route_with_tsr("/latest-release", get(latest_release))
        .route_with_tsr("/latest-ants", get(latest_ants))
        .route_with_tsr("/all-ants", get(all_ants))
        .fallback(|| async {
            return fallback::fallback(vec![
                "GET /latest-ants",
                "GET /all-ants",
                "GET /latest-release",
                "POST /suggest",
                "POST /tweet",
            ]);
        })
}
