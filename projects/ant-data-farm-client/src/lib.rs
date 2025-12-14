mod dao;

pub use crate::dao::dao_trait::DaoTrait;
pub use crate::dao::daos::ants;
use crate::dao::daos::api_tokens::ApiTokensDao;
pub use crate::dao::daos::hosts;
pub use crate::dao::daos::releases;
pub use crate::dao::daos::tweets;
pub use crate::dao::daos::users;
pub use crate::dao::daos::verifications;
pub use crate::dao::daos::web_actions;

use crate::dao::daos::{
    ants::AntsDao, hosts::HostsDao, releases::ReleasesDao, tweets::TweetsDao, users::UsersDao,
    verifications::VerificationsDao, web_actions::WebActionsDao,
};
use ant_library::db::database_connection;
use ant_library::db::Database;
use ant_library::db::DatabaseConfig;
use ant_library::db::TypesOfAntsDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

pub struct AntDataFarmClient {
    pub ants: RwLock<AntsDao>,
    pub releases: RwLock<ReleasesDao>,
    pub users: RwLock<UsersDao>,
    pub api_tokens: RwLock<ApiTokensDao>,
    pub verifications: RwLock<VerificationsDao>,
    pub tweets: RwLock<TweetsDao>,
    pub hosts: RwLock<HostsDao>,
    pub web_actions: RwLock<WebActionsDao>,
    // pub deployments: DeploymentsDao,
    // pub metrics: MetricsDao,
    // pub tests: TestsDao,
}

#[async_trait::async_trait]
impl TypesOfAntsDatabase for AntDataFarmClient {
    async fn connect(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        let pool = database_connection(&config).await?;

        info!("Initializing data access layer...");
        let database: Arc<Mutex<Database>> = Arc::new(Mutex::new(pool));

        Ok(AntDataFarmClient {
            ants: RwLock::new(AntsDao::new(database.clone()).await?),
            api_tokens: RwLock::new(ApiTokensDao::new(database.clone()).await?),
            releases: RwLock::new(ReleasesDao::new(database.clone()).await),
            tweets: RwLock::new(TweetsDao::new(database.clone())),
            users: RwLock::new(UsersDao::new(database.clone()).await?),
            verifications: RwLock::new(VerificationsDao::new(database.clone())),
            hosts: RwLock::new(HostsDao::new(database.clone()).await?),
            web_actions: RwLock::new(WebActionsDao::new(database.clone()).await?),
        })
    }
}

impl AntDataFarmClient {
    pub async fn connect_from_env(
        migration_dir: Option<PathBuf>,
    ) -> Result<AntDataFarmClient, anyhow::Error> {
        let cfg = DatabaseConfig {
            port: dotenv::var("ANT_DATA_FARM_PORT")?.parse()?,
            host: dotenv::var("ANT_DATA_FARM_HOST")?,
            database_name: ant_library::secret::load_secret("ant_data_farm_db")?,
            database_user: ant_library::secret::load_secret("ant_data_farm_user")?,
            database_password: ant_library::secret::load_secret("ant_data_farm_password")?,
            migration_dir: migration_dir,
        };

        AntDataFarmClient::connect(&cfg).await
    }
}
