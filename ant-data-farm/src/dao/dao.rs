use super::{
    dao_trait::DaoTrait,
    daos::{ants::AntsDao, hosts::HostsDao, releases::ReleasesDao, users::UsersDao},
};
use crate::{dao::db::Database, types::ConnectionPool};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct Dao {
    pub ants: RwLock<AntsDao>,
    pub releases: RwLock<ReleasesDao>,
    pub users: RwLock<UsersDao>,
    pub hosts: RwLock<HostsDao>,
    // pub deployments: DeploymentsDao,
    // pub metrics: MetricsDao,
    // pub tests: TestsDao,
}

impl Dao {
    pub async fn new(pool: ConnectionPool) -> Dao {
        let db_con: Database = pool
            .get_owned()
            .await
            .unwrap_or_else(|e| panic!("Failed to get a connection from pool: {e}"));

        let database: Arc<Mutex<Database>> = Arc::new(Mutex::new(db_con));

        let ants = RwLock::new(AntsDao::new(database.clone()).await);
        let releases = RwLock::new(ReleasesDao::new(database.clone()).await);
        let hosts = RwLock::new(HostsDao::new(database.clone()).await);
        let users = RwLock::new(UsersDao::new(database.clone()).await);

        Dao { ants, releases, users, hosts }
    }
}
