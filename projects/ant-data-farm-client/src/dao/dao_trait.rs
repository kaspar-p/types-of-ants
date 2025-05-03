use super::db::Database;
use crate::dao::daos::lib::Id;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
pub trait DaoTrait<K, T> {
    async fn new(db: Arc<Mutex<Database>>) -> anyhow::Result<K, anyhow::Error>;

    // Read
    async fn get_all(&self) -> anyhow::Result<Vec<T>>;
    async fn get_one_by_id(&self, id: &Id) -> anyhow::Result<Option<T>>;
}
