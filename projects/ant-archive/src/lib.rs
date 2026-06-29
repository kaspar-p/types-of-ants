pub mod auth;
pub mod err;
pub mod headers;
mod placement;
mod routes;
pub mod state;
mod storage_client;

pub use ant_archive_db::AntArchiveDb;
pub use auth::BearerClaims;
pub use axum::Router;
pub use err::AntArchiveError;
pub use state::AntArchiveState;

pub fn make_routes(state: AntArchiveState) -> Router {
    Router::new()
        .nest("/o", routes::objects::make_routes(state.clone()))
        .nest("/buckets", routes::buckets::make_routes(state))
}
