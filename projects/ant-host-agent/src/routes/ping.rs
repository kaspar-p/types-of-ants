use tracing::info;

pub async fn ping_route() -> &'static str {
    info!("Got health, responding with 'healthy ant'!");
    async { "healthy ant" }.await
}
