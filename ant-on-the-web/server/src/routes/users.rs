use crate::{
    middleware,
    types::{DaoRouter, DaoState},
};
use ant_data_farm::{users::User, DaoTrait};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::routing::RouterExt;
use serde::Deserialize;
use tracing::debug;

async fn create_user(State(_dao): DaoState) -> impl IntoResponse {
    (StatusCode::OK, Json("User created!"))
}

#[derive(Deserialize)]
struct EmailRequest {
    email: String,
}
async fn add_anonymous_email(
    State(dao): DaoState,
    Json(EmailRequest { email }): Json<EmailRequest>,
) -> impl IntoResponse {
    debug!("Subscribing with email {}", email.as_str());

    let exists = {
        let users = dao.users.read().await;
        users
            .get_one_by_name("nobody")
            .await
            .map(|u| u.emails.contains(&email))
    };
    match exists {
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to validate uniqueness").into_response(),
            )
        }
        Some(email_exists) => {
            if email_exists {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(format!("Email '{email}' is already subscribed!")).into_response(),
                );
            }
        }
    }

    let nobody_user: Option<User> = {
        let users = dao.users.read().await;
        users
            .get_one_by_name("nobody")
            .await
            .map(std::clone::Clone::clone)
    };
    if nobody_user.is_none() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("Failed attaching email!").into_response(),
        );
    }

    let mut user_write = dao.users.write().await;
    let user: Option<&User> = user_write
        .add_email_to_user(nobody_user.unwrap().user_id, email)
        .await;

    match user {
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("Failed attaching email!".to_owned()).into_response(),
        ),
        Some(_) => (
            StatusCode::OK,
            Json("Subscribed!".to_owned()).into_response(),
        ),
    }
}

async fn get_user_by_name(
    Path(user_name): Path<String>,
    State(dao): DaoState,
) -> impl IntoResponse {
    let users = dao.users.read().await;
    let user = users.get_one_by_name(&user_name).await;
    match user {
        None => (
            StatusCode::NOT_FOUND,
            Json(format!(
                "There was no user with username: {} found!",
                user_name
            ))
            .into_response(),
        ),
        Some(user) => (StatusCode::OK, Json(user).into_response()),
    }
}

pub fn router() -> DaoRouter {
    Router::new()
        .route_with_tsr("/subscribe-newsletter", post(add_anonymous_email))
        .route_with_tsr("/user", post(create_user))
        .route_with_tsr("/user/:user-name", get(get_user_by_name))
        .fallback(|| async {
            middleware::fallback(&[
                "POST /user",
                "POST /user/:user-name",
                "POST /subscribe-newsletter",
            ])
        })
}
