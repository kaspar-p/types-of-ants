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
use ant_library::db::{
    database_connection, database_connection_dynamic, ConnectionPool, DatabaseConfig,
    DatabaseCredentialsConfig, TypesOfAntsDatabase,
};
use ant_library::sd::reader::ServiceDiscovery;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

pub struct AntDataFarmClient {
    pub ants: AntsDao,
    pub releases: ReleasesDao,
    pub users: UsersDao,
    pub api_tokens: ApiTokensDao,
    pub verifications: VerificationsDao,
    pub tweets: TweetsDao,
    pub hosts: HostsDao,
    pub web_actions: WebActionsDao,
    // pub deployments: DeploymentsDao,
    // pub metrics: MetricsDao,
    // pub tests: TestsDao,
}

#[async_trait::async_trait]
impl TypesOfAntsDatabase for AntDataFarmClient {
    async fn connect(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        let pool = database_connection(config).await?;
        Self::build_from_pool(pool).await
    }
}

impl AntDataFarmClient {
    pub async fn connect_from_env(
        migration_dirs: Vec<PathBuf>,
    ) -> Result<AntDataFarmClient, anyhow::Error> {
        let cfg = DatabaseConfig {
            port: dotenv::var("ANT_DATA_FARM_PORT")?.parse()?,
            host: dotenv::var("ANT_DATA_FARM_HOST")?,
            database_name: ant_library::secret::load_secret("ant_data_farm_db")?,
            database_user: ant_library::secret::load_secret("ant_data_farm_user")?,
            database_password: ant_library::secret::load_secret("ant_data_farm_password")?,
            migration_dirs,
        };

        AntDataFarmClient::connect(&cfg).await
    }

    /// Connect via Consul service discovery. The pool re-resolves "ant-data-farm" on every
    /// new connection and recycles connections automatically when the endpoint changes.
    pub async fn connect_discovered(
        sd: Arc<ServiceDiscovery>,
        migration_dirs: Vec<PathBuf>,
    ) -> Result<AntDataFarmClient, anyhow::Error> {
        let creds = DatabaseCredentialsConfig {
            database_name: ant_library::secret::load_secret("ant_data_farm_db")?,
            database_user: ant_library::secret::load_secret("ant_data_farm_user")?,
            database_password: ant_library::secret::load_secret("ant_data_farm_password")?,
            migration_dirs,
        };

        let pool = database_connection_dynamic(sd, "ant-data-farm", &creds).await?;
        Self::build_from_pool(pool).await
    }

    async fn build_from_pool(pool: ConnectionPool) -> Result<AntDataFarmClient, anyhow::Error> {
        info!("Initializing data access layer...");
        let pool: Arc<ConnectionPool> = Arc::new(pool);

        Ok(AntDataFarmClient {
            ants: AntsDao::new(pool.clone()).await?,
            api_tokens: ApiTokensDao::new(pool.clone()).await?,
            releases: ReleasesDao::new(pool.clone()).await,
            tweets: TweetsDao::new(pool.clone()),
            users: UsersDao::new(pool.clone()).await?,
            verifications: VerificationsDao::new(pool.clone()),
            hosts: HostsDao::new(pool.clone()).await?,
            web_actions: WebActionsDao::new(pool.clone()).await?,
        })
    }
}
