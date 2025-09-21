use crate::{
    err::{AntOnTheWebError, ValidationError, ValidationMessage},
    routes::lib::auth::{admin_authenticate, authenticate, optional_strict_authenticate},
    state::{ApiRouter, ApiState, InnerApiState},
};
use ant_data_farm::users::UserId;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::lib::auth::{optional_authenticate, AuthClaims};

pub use ant_data_farm::ants::{Ant, AntId, AntStatus};
pub use ant_data_farm::releases::{AntReleaseRequest, Release};

const PAGE_SIZE: usize = 1_000_usize;

#[derive(Serialize, Deserialize)]
pub struct AllAntsResponse {
    pub ants: Vec<Ant>,
}
async fn all_ants(
    State(InnerApiState { dao, .. }): ApiState,
) -> Result<impl IntoResponse, AntOnTheWebError> {
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

#[derive(Serialize, Deserialize)]
pub struct UnreleasedAntsResponse {
    pub ants: Vec<Ant>,
}

async fn unreleased_ants(
    State(InnerApiState { dao, .. }): ApiState,
    query: Query<Pagination>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
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

#[derive(Serialize, Deserialize)]
pub struct DeclinedAntsRequest {
    pub page: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclinedAntsResponse {
    pub ants: Vec<Ant>,
    pub has_next_page: bool,
}

async fn declined_ants(
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<DeclinedAntsRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let ants = dao.ants.read().await;

    let declined_ants = ants
        .get_all()
        .await?
        .into_iter()
        .filter_map(|ant| match ant.status {
            AntStatus::Declined => Some(ant),
            _ => None,
        })
        .collect::<Vec<Ant>>();

    let mut chunks = declined_ants.chunks(PAGE_SIZE);
    let ants_page = chunks.nth(req.page.try_into().unwrap());

    match ants_page {
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                Json(format!("No page {} exists!", req.page)).into_response(),
            ))
        }
        Some(declined_ants) => {
            return Ok((
                StatusCode::OK,
                Json(DeclinedAntsResponse {
                    ants: declined_ants.to_vec(),
                    has_next_page: chunks.len() > 0,
                })
                .into_response(),
            ));
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReleasedAnt {
    pub ant_id: AntId,
    pub ant_name: String,
    pub hash: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub created_by_username: String,
    pub release: Release,

    /// If the user is logged in, this is Some(bool), else None
    pub favorited_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasedAntsResponse {
    pub ants: Vec<ReleasedAnt>,
    pub has_next_page: bool,
}
async fn released_ants(
    auth: Option<AuthClaims>,
    State(InnerApiState { dao, .. }): ApiState,
    query: Query<Pagination>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = optional_strict_authenticate(auth.as_ref(), &dao).await?;

    let ants = dao.ants.read().await;

    let released_ants = match &user {
        None => ants.get_all().await?,
        Some(u) => ants.get_all_with_user_context(&u).await?,
    }
    .into_iter()
    .filter_map(|ant| match ant.status {
        AntStatus::Released(release) => Some(ReleasedAnt {
            ant_id: ant.ant_id,
            ant_name: ant.ant_name,
            created_at: ant.created_at,
            created_by: ant.created_by,
            created_by_username: ant.created_by_username,

            hash: ant.hash,
            release: release,

            favorited_at: ant.favorited_at,
        }),
        _ => None,
    })
    .collect::<Vec<ReleasedAnt>>();

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct LatestReleaseResponse {
    pub release: Release,
}
async fn latest_release(State(InnerApiState { dao, .. }): ApiState) -> impl IntoResponse {
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
pub struct GetReleaseRequest {
    pub release: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct GetReleaseResponse {
    pub release: Release,
}

async fn get_release(
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<GetReleaseRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let releases = dao.releases.read().await;
    let release = releases.get_release(req.release).await?;

    match release {
        None => Ok((StatusCode::NOT_FOUND).into_response()),
        Some(release) => Ok((StatusCode::OK, Json(GetReleaseResponse { release })).into_response()),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateReleaseRequest {
    pub label: String,
    pub ants: Vec<AntReleaseRequest>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateReleaseResponse {
    pub release: i32,
}

async fn create_release(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<CreateReleaseRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = admin_authenticate(&auth, &dao).await?;

    {
        let mut validations: Vec<ValidationMessage> = vec![];
        if req.ants.len() == 0 {
            validations.push(ValidationMessage::new("ants", "Ants cannot be empty."));
        }

        if req.ants.len() > 256 {
            validations.push(ValidationMessage::new(
                "ants",
                "Ants too long, cannot exceed 256.",
            ));
        }

        let ants = dao.ants.read().await;
        for ant_req in &req.ants {
            if req
                .ants
                .iter()
                .filter(|&ant| ant.ant_id == ant_req.ant_id)
                .count()
                > 1
            {
                validations.push(ValidationMessage::new(
                    "ants",
                    format!("Ant {} suggested more than once.", ant_req.ant_id),
                ));
            }

            let ant = ants.get_one_by_id(&ant_req.ant_id).await?;
            match ant {
                None => {
                    validations.push(ValidationMessage::new(
                        "ants",
                        format!("No such ant: {}", ant_req.ant_id),
                    ));
                }
                Some(ant) => match ant.status {
                    AntStatus::Unreleased => {}
                    status => validations.push(ValidationMessage::new(
                        "ants",
                        format!(
                            "Only unreleased ants may be suggested, ant {} is {}",
                            ant_req.ant_id, status
                        ),
                    )),
                },
            }

            if let Some(content) = &ant_req.overwrite_content {
                validate_suggested_content(&content).map(|e| validations.push(e));
            }
        }

        if validations.len() > 0 {
            return Err(AntOnTheWebError::Validation(ValidationError::many(
                validations,
            )));
        }
    }

    let mut releases = dao.releases.write().await;

    let release = releases
        .make_release(&user.user_id, req.label, req.ants)
        .await?;

    Ok((StatusCode::OK, Json(CreateReleaseResponse { release })))
}

#[derive(Serialize, Deserialize)]
pub struct TotalResponse {
    pub total: usize,
}
async fn total(
    State(InnerApiState { dao, .. }): ApiState,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let ants = dao.ants.read().await;
    let total = ants.get_all_released().await?.len();
    Ok((StatusCode::OK, Json(TotalResponse { total })))
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct LatestAntsResponse {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub date: chrono::DateTime<chrono::Utc>,
    pub release: i32,
    pub ants: Vec<Ant>,
}
async fn latest_ants(
    State(InnerApiState { dao, .. }): ApiState,
) -> Result<impl IntoResponse, AntOnTheWebError> {
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
                .filter(|ant| match &ant.status {
                    AntStatus::Released(n) => n.release_number == latest_release.release_number,
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
pub struct FeedResponse {
    pub ants: Vec<Ant>,
}

async fn feed(
    State(InnerApiState { dao, .. }): ApiState,
    query: Query<FeedInput>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let ants = dao.ants.read().await;

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

fn validate_suggested_content(content: &str) -> Option<ValidationMessage> {
    if content.len() < 3 || content.len() > 100 {
        return Some(ValidationMessage::new(
            "content",
            "Ant content must be between 3 and 100 characters.",
        ));
    }

    None
}

async fn make_suggestion(
    auth: Option<AuthClaims>,
    State(InnerApiState { dao, .. }): ApiState,
    Json(suggestion): Json<SuggestionRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = optional_authenticate(auth.as_ref(), &dao).await?;

    {
        let mut validations: Vec<ValidationMessage> = vec![];
        validate_suggested_content(&suggestion.suggestion_content).map(|v| validations.push(v));

        if validations.len() > 0 {
            return Err(AntOnTheWebError::Validation(ValidationError::many(
                validations,
            )));
        }
    }

    let mut ants = dao.ants.write().await;

    let ant = ants
        .add_unreleased_ant(suggestion.suggestion_content, user.user_id, user.username)
        .await?;

    Ok((
        StatusCode::OK,
        Json(SuggestionResponse { ant }).into_response(),
    ))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclineAntRequest {
    pub ant_id: AntId,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclineAntResponse {
    pub declined_at: DateTime<Utc>,
}

async fn decline_ant(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<DeclineAntRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = admin_authenticate(&auth, &dao).await?;

    let mut ants = dao.ants.write().await;

    let declined_at = match ants.get_one_by_id(&req.ant_id).await? {
        None => {
            return Err(AntOnTheWebError::Validation(ValidationError::one(
                ValidationMessage::new("ant_id", "No such ant."),
            )));
        }
        Some(ant) => match ant.status {
            AntStatus::Declined => Err(AntOnTheWebError::Validation(ValidationError::one(
                ValidationMessage::new("ant_id", format!("Ant already declined.")),
            ))),
            AntStatus::Released(_) => Err(AntOnTheWebError::Validation(ValidationError::one(
                ValidationMessage::new("ant_id", format!("Ant already released.")),
            ))),
            AntStatus::Unreleased => {
                let declined_at = ants.decline_ant(&user.user_id, &ant.ant_id).await?;

                Ok(declined_at)
            }
        },
    }?;

    return Ok((StatusCode::OK, Json(DeclineAntResponse { declined_at })));
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteAntRequest {
    pub ant_id: AntId,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteAntResponse {
    pub favorited_at: DateTime<Utc>,
}

async fn favorite_ant(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<FavoriteAntRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = authenticate(&auth, &dao).await?;

    let mut ants = dao.ants.write().await;

    if ants.get_one_by_id(&req.ant_id).await?.is_none() {
        return Err(AntOnTheWebError::Validation(ValidationError::one(
            ValidationMessage::new("antId", "No such ant."),
        )));
    }

    let favorited_at: DateTime<Utc> = match ants.is_favorite_ant(&user.user_id, &req.ant_id).await?
    {
        Some(time) => time,
        None => ants.favorite_ant(&user.user_id, &req.ant_id).await?,
    };

    Ok((StatusCode::OK, Json(FavoriteAntResponse { favorited_at })))
}

#[derive(Serialize, Deserialize)]
pub struct UnfavoriteAntRequest {
    #[serde(rename = "antId")]
    pub ant_id: AntId,
}

async fn unfavorite_ant(
    auth: AuthClaims,
    State(InnerApiState { dao, .. }): ApiState,
    Json(req): Json<UnfavoriteAntRequest>,
) -> Result<impl IntoResponse, AntOnTheWebError> {
    let user = authenticate(&auth, &dao).await?;

    let mut ants = dao.ants.write().await;

    if ants.get_one_by_id(&req.ant_id).await?.is_none() {
        return Err(AntOnTheWebError::Validation(ValidationError::one(
            ValidationMessage::new("antId", "No such ant."),
        )));
    }

    if ants
        .is_favorite_ant(&user.user_id, &req.ant_id)
        .await?
        .is_some()
    {
        ants.unfavorite_ant(&user.user_id, &req.ant_id).await?;
    };

    Ok((StatusCode::OK, "Ant unfavorited."))
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

pub fn router() -> ApiRouter {
    Router::new()
        .route_with_tsr("/feed", get(feed))
        .route_with_tsr("/latest-ants", get(latest_ants))
        .route_with_tsr("/unreleased-ants", get(unreleased_ants))
        .route_with_tsr("/released-ants", get(released_ants))
        .route_with_tsr("/declined-ants", get(declined_ants))
        .route_with_tsr("/all-ants", get(all_ants))
        .route_with_tsr("/release", get(get_release).post(create_release))
        .route_with_tsr("/latest-release", get(latest_release))
        .route_with_tsr("/total", get(total))
        .route_with_tsr("/suggest", post(make_suggestion))
        .route_with_tsr("/decline", post(decline_ant))
        .route_with_tsr("/favorite", post(favorite_ant))
        .route_with_tsr("/unfavorite", post(unfavorite_ant))
        // .route_with_tsr("/tweet", post(tweet))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "GET /feed",
                "GET /latest-ants",
                "GET /unreleased-ants",
                "GET /released-ants",
                "GET /declined-ants",
                "GET /all-ants",
                "GET+POST /release",
                "GET /latest-release",
                "GET /total",
                "POST /suggest",
                "POST /decline",
                "POST /favorite",
                "POST /unfavorite",
                // "POST /tweet",
            ])
        })
}
