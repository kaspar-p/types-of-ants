use crate::dao::dao::Dao;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

async fn create_user(State(dao): State<Arc<Dao>>) -> impl IntoResponse {
    (StatusCode::OK, Json("User created!"))
}

async fn get_user_by_name(
    Path(user_name): Path<String>,
    State(dao): State<Arc<Dao>>,
) -> impl IntoResponse {
    return (
        StatusCode::NOT_FOUND,
        Json(format!(
            "User with name '{}' doesn't exist! This is probably because there are currently no users at all.",
            user_name
        )),
    );
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new()
        .route("/user", post(create_user))
        .route("/user/:user-name", get(get_user_by_name))
}
