use axum::Router;

use crate::state::ApiRouter;

pub fn router() -> ApiRouter {
    Router::new()
}
