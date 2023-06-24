use std::sync::Arc;

use crate::dao::{dao::Dao, daos::ants::Ant};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AllAntsResponse {
    ants: Vec<Ant>,
}
async fn all_ants(State(dao): State<Arc<Dao>>) -> impl IntoResponse {
    let ants: Vec<Ant> = dao.ants.get_all_ants();
    (StatusCode::OK, Json(AllAntsResponse { ants }))
}

#[derive(Serialize, Deserialize)]
struct LatestAntsResponse {
    #[serde(with = "chrono::serde::ts_seconds")]
    date: chrono::DateTime<chrono::Utc>,
    ants: Vec<Ant>,
}

async fn current_release(State(dao): State<Arc<Dao>>) -> impl IntoResponse {
    let release = dao.ants.get_current_release();
    (StatusCode::OK, Json(release))
}

async fn latest_ants(State(dao): State<Arc<Dao>>) -> impl IntoResponse {
    let ants: Vec<Ant> = dao.ants.get_all_ants();
    (
        StatusCode::OK,
        Json(LatestAntsResponse {
            date: chrono::offset::Utc::now(),
            ants,
        }),
    )
}

pub fn router() -> Router<Arc<Dao>> {
    Router::new()
        .route("/current-release", get(current_release))
        .route("/latest-ants", get(latest_ants))
        .route("/all-ants", get(all_ants))
}
