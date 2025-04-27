mod routes;

use ant_owning_artifacts::start_server;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    start_server(None).await?;

    Ok(())
}
