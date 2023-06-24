use super::daos::{ants::AntsDao, hosts::HostsDao, users::UsersDao};
use crate::{dao::db::Database, lib::ConnectionPool};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Dao {
    pub ants: AntsDao,
    pub users: UsersDao,
    pub hosts: HostsDao,
    // pub deployments: DeploymentsDao,
    // pub metrics: MetricsDao,
    // pub tests: TestsDao,
}

impl Dao {
    pub async fn new(pool: ConnectionPool) -> Dao {
        let db_con: Database = pool
            .get_owned()
            .await
            .unwrap_or_else(|e| panic!("Failed to get a connection from pool: {}", e));

        let database: Arc<Mutex<Database>> = Arc::new(Mutex::new(db_con));

        let ants = AntsDao::new(database.clone()).await;
        let hosts = HostsDao::new(database.clone()).await;
        let users = UsersDao::new(database.clone()).await;

        Dao { ants, hosts, users }
    }
}
