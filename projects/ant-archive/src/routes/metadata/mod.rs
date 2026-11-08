use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;

use crate::AntArchiveState;

pub mod list_objects;

pub fn make_routes(state: AntArchiveState) -> Router {
    use ant_library::routes::Routes;

    Routes::new()
        .get(
            "/objects/{bucket_id}/{*key}",
            get(list_objects::list_objects),
        )
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
                ))),
        )
}
