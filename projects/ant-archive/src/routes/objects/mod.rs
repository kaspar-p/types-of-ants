use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, put},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{catch_panic::CatchPanicLayer, limit::RequestBodyLimitLayer};

use crate::AntArchiveState;

pub mod delete_object;
pub mod get_object;
mod kek;
pub mod put_object;
mod tek;

pub fn make_routes(state: AntArchiveState) -> Router {
    use ant_library::routes::Routes;

    Routes::new()
        .put("/{bucket_id}/{*key}", put(put_object::put_object))
        .get("/{bucket_id}/{*key}", get(get_object::get_object))
        .delete("/{bucket_id}/{*key}", delete(delete_object::delete_object))
        .build()
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                )))
                .layer(DefaultBodyLimit::disable())
                .layer(RequestBodyLimitLayer::new(1024 * 1024 * 1024)),
        )
}
