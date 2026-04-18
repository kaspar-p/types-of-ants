use crate::dao::daos::lib::Id;
use ant_library::db::ConnectionPool;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait DaoTrait<K, T> {
    async fn new(db: Arc<ConnectionPool>) -> anyhow::Result<K, anyhow::Error>;

    // Read
    async fn get_all(&self) -> anyhow::Result<Vec<T>>;
    async fn get_one_by_id(&self, id: &Id) -> anyhow::Result<Option<T>>;
}
