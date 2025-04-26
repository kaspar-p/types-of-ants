use crate::types::{DbRouter, DbState};
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
use uuid::Uuid;

#[derive(Deserialize)]
struct EmailRequest {
    email: String,
}
async fn add_anonymous_email(
    State(dao): DbState,
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

async fn get_user_by_name(Path(user_name): Path<String>, State(dao): DbState) -> impl IntoResponse {
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

#[derive(Deserialize)]
struct LoginRequest {
    // The user can pass either a username, or phone number,
    // or email, all of which are unique to them
    pub unique_key: String,
    pub cookie: Uuid,
}
async fn login(State(dao): DbState, Json(login_request): Json<LoginRequest>) -> impl IntoResponse {
    let users = dao.users.read().await;
    let from_email = users.get_one_by_email(&login_request.unique_key).await;
    let from_phone_number = users
        .get_one_by_phone_number(&login_request.unique_key)
        .await;
    let from_username = users.get_one_by_name(&login_request.unique_key).await;

    let all_some = [from_email, from_phone_number, from_username]
        .iter()
        .fold(true, |acc, &u| acc && u.is_some());
    if !all_some {
        return (
            StatusCode::BAD_REQUEST,
            Json("No user exists with that username, phone number, or email!").into_response(),
        );
    }

    let all_same =
        [from_email, from_phone_number, from_username]
            .iter()
            .fold(from_email, |acc, &u| {
                if let (Some(prev), Some(current)) = (acc, u) {
                    if current == prev {
                        return Some(current);
                    }
                }
                return None;
            });

    // Check the cookie is valid for that user. If it is, allow them through. If not, do two-factor

    match all_same {
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("ERROR: encountered error parsing user data").into_response(),
        ),
        Some(u) => (
            StatusCode::OK,
            Json("Found user, logging you in!").into_response(), // TODO: give the user a cookie to save!
        ),
    }
}

#[derive(Deserialize)]
struct VerificationCode(String);

#[derive(Deserialize)]
struct SignupVerificationRequest {
    pub username: String,
    pub email: String,
    pub phone_number: String,
    pub phone_verification: VerificationCode,
    pub email_verification: VerificationCode,
}
async fn signup_verification(
    State(db): DbState,
    Json(signup_verification_request): Json<SignupVerificationRequest>,
) -> impl IntoResponse {
    todo!("Verify that the verification codes sent are correct!");

    let mut write_users = db.users.write().await;
    let user = write_users
        .create_user(
            signup_verification_request.username,
            signup_verification_request.phone_number,
            vec![signup_verification_request.email],
        )
        .await;

    match user {
        Some(_) => (
            StatusCode::OK,
            Json("Signup request accepted!").into_response(),
        ),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json("ERROR: Signup request failed!").into_response(),
        ),
    }
}

#[derive(Deserialize)]
struct SignupRequest {
    pub username: String,
    pub email: String,
    pub phone_number: String,
}
async fn signup_request(
    State(dao): DbState,
    Json(signup_request): Json<SignupRequest>,
) -> impl IntoResponse {
    let read_users = dao.users.read().await;

    // Check if the user already exists
    let by_username = read_users
        .get_one_by_name(signup_request.username.as_str())
        .await;
    let by_email = read_users
        .get_one_by_email(signup_request.email.as_str())
        .await;
    let by_phone_number = read_users
        .get_one_by_phone_number(signup_request.phone_number.as_str())
        .await;
    for user in [by_username, by_email, by_phone_number] {
        if user.is_some() {
            return (
                StatusCode::BAD_REQUEST,
                Json(format!("A user with that data already exists!")).into_response(),
            );
        }
    }

    todo!(
        "Send verification codes to the user!
    1. Instantiate a twilio client here and call it
    2. Instantiate an email client and call it!"
    );
}

pub fn router() -> DbRouter {
    Router::new()
        .route_with_tsr("/subscribe-newsletter", post(add_anonymous_email))
        .route_with_tsr("/signup-request", post(signup_request))
        .route_with_tsr("/signup-verification", post(signup_verification))
        .route_with_tsr("/login", post(login))
        .route_with_tsr("/user/:user-name", get(get_user_by_name))
        .fallback(|| async {
            ant_library::api_fallback(&[
                "POST /signup",
                "POST /user/:user-name",
                "POST /subscribe-newsletter",
            ])
        })
}
