use axum::Json;
use tracing::info;

pub async fn ping_route() -> String {
    info!("Got health, responding with 'healthy ant'!");
    async { "healthy ant".to_owned() }.await
}
