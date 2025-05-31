use crate::types::{DbRouter, DbState};
use ant_data_farm::{
    ants::{Ant, AntId, AntStatus},
    releases::Release,
    users::UserId,
    DaoTrait,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use super::lib::auth::{optional_authenticate, AuthClaims, AuthError};

const PAGE_SIZE: usize = 1_000_usize;

#[derive(Serialize, Deserialize)]
pub struct AllAntsResponse {
    pub ants: Vec<Ant>,
}
async fn all_ants(State(dao): DbState) -> Result<impl IntoResponse, AntsError> {
    let ants = dao.ants.read().await.get_all().await?;
    Ok((
        StatusCode::OK,
        Json(AllAntsResponse { ants }).into_response(),
    ))
}

#[derive(Serialize, Deserialize)]
struct Pagination {
    page: usize,
}

#[derive(Serialize)]
struct UnreleasedAntsResponse {
    pub ants: Vec<Ant>,
}

async fn unreleased_ants(
    State(dao): DbState,
    query: Query<Pagination>,
) -> Result<impl IntoResponse, AntsError> {
    let ants = dao.ants.read().await;

    let mut unreleased_ants = ants
        .get_all()
        .await?
        .into_iter()
        .filter_map(|ant| match ant.status {
            AntStatus::Unreleased => Some(ant),
            _ => None,
        })
        .collect::<Vec<Ant>>();
    unreleased_ants.sort();
    unreleased_ants.reverse();

    let ants_page = unreleased_ants.chunks(PAGE_SIZE).nth(query.page);
    match ants_page {
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                Json(format!("No page {} exists!", query.page)).into_response(),
            ))
        }
        Some(unreleased_ants) => {
            return Ok((
                StatusCode::OK,
                Json(UnreleasedAntsResponse {
                    ants: unreleased_ants.to_vec(),
                })
                .into_response(),
            ));
        }
    }
}

async fn declined_ants(
    State(dao): DbState,
    query: Query<Pagination>,
) -> Result<impl IntoResponse, AntsError> {
    let ants = dao.ants.read().await;

    let declined_ants = ants
        .get_all()
        .await?
        .into_iter()
        .filter_map(|ant| match ant.status {
            AntStatus::Released(_) => Some(ant),
            _ => None,
        })
        .collect::<Vec<Ant>>();

    let ants_page = declined_ants.chunks(PAGE_SIZE).nth(query.page);
    match ants_page {
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                Json(format!("No page {} exists!", query.page)).into_response(),
            ))
        }
        Some(released_ants) => {
            return Ok((StatusCode::OK, Json(released_ants).into_response()));
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ReleasedAntsResponse {
    pub ants: Vec<Ant>,

    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}
async fn released_ants(
    State(dao): DbState,
    query: Query<Pagination>,
) -> Result<impl IntoResponse, AntsError> {
    let ants = dao.ants.read().await;

    let released_ants = ants
        .get_all()
        .await?
        .into_iter()
        .filter_map(|ant| match ant.status {
            AntStatus::Released(_) => Some(ant),
            _ => None,
        })
        .collect::<Vec<Ant>>();

    let mut chunks = released_ants.chunks(PAGE_SIZE);

    let has_next_page = (chunks.len() - 1) > query.page;
    let ants_page = chunks.nth(query.page).unwrap_or(&[]);

    return Ok((
        StatusCode::OK,
        Json(ReleasedAntsResponse {
            ants: ants_page.to_vec(),
            has_next_page,
        })
        .into_response(),
    ));
}

#[derive(Serialize, Deserialize)]
struct LatestReleaseResponse {
    release: Release,
}
async fn latest_release(State(dao): DbState) -> impl IntoResponse {
    match dao.releases.read().await.get_latest_release().await {
        Err(_) => (StatusCode::NOT_FOUND).into_response(),
        Ok(latest_release) => (
            StatusCode::OK,
            Json(LatestReleaseResponse {
                release: latest_release,
            }),
        )
            .into_response(),
    }
}

#[derive(Serialize, Deserialize)]
pub struct TotalResponse {
    pub total: usize,
}
async fn total(State(dao): DbState) -> Result<impl IntoResponse, AntsError> {
    let ants = dao.ants.read().await;
    let total = ants.get_all_released().await?.len();
    Ok((StatusCode::OK, Json(TotalResponse { total })))
}

#[derive(Serialize, Deserialize)]
struct LatestAntsResponse {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub date: chrono::DateTime<chrono::Utc>,
    pub release: i32,
    pub ants: Vec<Ant>,
}
async fn latest_ants(State(dao): DbState) -> Result<impl IntoResponse, AntsError> {
    let ants = dao.ants.read().await;
    let releases = dao.releases.read().await;

    let all_ants: Vec<Ant> = ants.get_all().await?;
    match releases.get_latest_release().await {
        Err(_) => {
            return Ok((StatusCode::NOT_FOUND).into_response());
        }
        Ok(latest_release) => {
            let current_release_ants = all_ants
                .iter()
                .filter(|ant| match ant.status {
                    AntStatus::Released(n) => n == latest_release.release_number,
                    _ => false,
                })
                .map(std::clone::Clone::clone)
                .collect::<Vec<Ant>>();

            return Ok((
                StatusCode::OK,
                Json(LatestAntsResponse {
                    date: latest_release.created_at,
                    release: latest_release.release_number,
                    ants: current_release_ants,
                }),
            )
                .into_response());
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FeedInput {
    #[serde(rename = "userId")]
    pub user_id: UserId,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub since: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct FeedResponse {
    pub ants: Vec<Ant>,
}

async fn feed(State(db): DbState, query: Query<FeedInput>) -> Result<impl IntoResponse, AntsError> {
    let ants = db.ants.read().await;

    let feed = match ants
        .get_user_feed_since(&query.user_id, &query.since)
        .await?
    {
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                Json("User does not exist!".to_string()).into_response(),
            ))
        }
        Some(feed) => feed,
    };

    return Ok((
        StatusCode::OK,
        Json(FeedResponse { ants: feed }).into_response(),
    ));
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SuggestionRequest {
    #[serde(rename = "suggestionContent")]
    pub suggestion_content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SuggestionResponse {
    pub ant: Ant,
}

async fn make_suggestion(
    auth: Option<AuthClaims>,
    State(dao): DbState,
    Json(suggestion): Json<SuggestionRequest>,
) -> Result<impl IntoResponse, AntsError> {
    let user = optional_authenticate(auth.as_ref(), &dao).await?;

    let mut ants = dao.ants.write().await;

    let ant = ants
        .add_unreleased_ant(suggestion.suggestion_content, user.user_id, user.username)
        .await?;

    Ok((
        StatusCode::OK,
        Json(SuggestionResponse { ant }).into_response(),
    ))
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
        .route_with_tsr("/feed", get(feed))
        .route_with_tsr("/latest-ants", get(latest_ants))
        .route_with_tsr("/unreleased-ants", get(unreleased_ants))
        .route_with_tsr("/released-ants", get(released_ants))
        .route_with_tsr("/declined-ants", get(declined_ants))
        .route_with_tsr("/all-ants", get(all_ants))
        .route_with_tsr("/latest-release", get(latest_release))
        .route_with_tsr("/total", get(total))
        .route_with_tsr("/suggest", post(make_suggestion))
        // .route_with_tsr("/tweet", post(tweet))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "GET /feed",
                "GET /latest-ants",
                "GET /unreleased-ants",
                "GET /released-ants",
                "GET /declined-ants",
                "GET /all-ants",
                "GET /latest-release",
                "GET /total",
                "POST /suggest",
                // "POST /tweet",
            ])
        })
}

enum AntsError {
    AccessDenied(Option<String>),
    InternalServerError(anyhow::Error),
}

impl IntoResponse for AntsError {
    fn into_response(self) -> Response {
        match self {
            AntsError::InternalServerError(e) => {
                error!("AntsError::InternalServerError: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong, please retry",
                )
                    .into_response()
            }
            AntsError::AccessDenied(identity) => {
                warn!("Access denied to identity: {:?}", identity);
                (StatusCode::UNAUTHORIZED, "Access denied.").into_response()
            }
        }
    }
}

impl From<AuthError> for AntsError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::AccessDenied(identity) => AntsError::AccessDenied(identity),
            AuthError::InternalServerError(e) => AntsError::InternalServerError(e),
        }
    }
}

impl<E> From<E> for AntsError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(err.into())
    }
}
