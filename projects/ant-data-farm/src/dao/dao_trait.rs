use super::db::Database;
use crate::dao::daos::lib::Id;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
pub trait DaoTrait<K, T> {
    async fn new(db: Arc<Mutex<Database>>) -> anyhow::Result<K, anyhow::Error>;

    // Read
    async fn get_all(&self) -> Vec<&T>;
    async fn get_all_mut(&mut self) -> Vec<&mut T>;
    async fn get_one_by_id(&self, id: &Id) -> Option<&T>;
    async fn get_one_by_id_mut(&mut self, id: &Id) -> Option<&mut T>;
    async fn get_one_by_name(&self, name: &str) -> Option<&T>;
    async fn get_one_by_name_mut(&mut self, name: &str) -> Option<&mut T>;
}
