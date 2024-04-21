use axum::Json;
use tracing::info;

pub async fn ping_route() -> Json<String> {
    info!("Got health, responding with 'healthy ant'!");
    async { Json("healthy ant".to_owned()) }.await
}
