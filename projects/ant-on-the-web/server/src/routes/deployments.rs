use axum::Router;

use crate::types::ApiRouter;

pub fn router() -> ApiRouter {
    Router::new()
}
