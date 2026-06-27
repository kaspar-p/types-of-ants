use std::env::args;

use ant_library::db::{DatabaseConfig, TypesOfAntsDatabase};
use ant_zookeeper::pipeline_engine::engine::PipelineEngine;
use ant_zookeeper_db::AntZooStorageClient;
use anyhow::Context;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let db = AntZooStorageClient::connect(&DatabaseConfig {
        port: std::env::var("ANT_ZOOKEEPER_DB_PORT")
            .context("ANT_ZOOKEEPER_DB_PORT")?
            .parse()?,
        database_name: ant_library::secret::load_secret("ant_zookeeper_db_db")?,
        database_password: ant_library::secret::load_secret("ant_zookeeper_db_password")?,
        database_user: ant_library::secret::load_secret("ant_zookeeper_db_user")?,
        host: std::env::var("ANT_ZOOKEEPER_DB_HOST")
            .context("ANT_ZOOKEEPER_DB_HOST")?
            .parse()?,
        migration_dirs: vec![],
    })
    .await?;

    let engine = PipelineEngine::new(db.pool()).await?;

    let mut args = args();
    args.next();

    let node_id = args.next().expect("first arg undefined");

    let reason = engine.why_blocked(&node_id).await?;

    println!("BLOCKED BECAUSE:");
    println!("{}", serde_json::to_string_pretty(&reason)?);

    Ok(())
}
