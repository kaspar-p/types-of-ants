use std::sync::{Arc, Mutex};

use ant_library::db::{Database, DatabaseConfig, TypesOfAntsDatabase, database_connection};
use async_trait::async_trait;

#[derive(Clone)]
pub struct AntZooStorageClient {
    database: Arc<Mutex<Database>>,
}

#[async_trait]
impl TypesOfAntsDatabase for AntZooStorageClient {
    async fn connect(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        let database = database_connection(&config).await?;

        Ok(AntZooStorageClient {
            database: Arc::new(Mutex::new(database)),
        })
    }
}
