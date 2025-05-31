use axum::Router;
use axum_extra::routing::RouterExt;

use crate::types::DbRouter;

pub fn router() -> DbRouter {
    Router::new()
}
