use std::{collections::HashMap, fs::remove_dir_all, path::PathBuf, sync::Arc};

use ant_host_agent::{
    client::{AntHostAgentClient, AntHostAgentClientConfig, AntHostAgentClientFactory},
    state::AntHostAgentState,
};
use ant_library::db::TypesOfAntsDatabase;
use ant_library_test::{axum_test_client::TestClient, db::test_database_config};
use ant_zoo_storage::AntZooStorageClient;
use ant_zookeeper::{
    dns::{Dns, TxtRecord},
    make_routes,
    state::AntZookeeperState,
};
use async_trait::async_trait;
use chrono::Utc;
use rsa::rand_core::OsRng;
use tokio::{fs::create_dir_all, net::TcpListener, sync::Mutex, task::JoinHandle};

struct TestDns {
    records: Mutex<HashMap<String, Vec<TxtRecord>>>,
}

impl TestDns {
    pub fn new() -> Self {
        TestDns {
            records: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Dns for TestDns {
    async fn put_txt_record(
        &self,
        domain: &str,
        val: String,
    ) -> Result<ant_zookeeper::dns::TxtRecord, anyhow::Error> {
        if let Some(record) = self
            .list_txt_records(domain)
            .await?
            .iter()
            .find(|record| record.content == val)
        {
            return Ok(record.clone());
        }

        let record = TxtRecord {
            id: Utc::now().to_rfc3339(),
            content: val,
        };
        let mut domain_records = self.records.lock().await;

        domain_records
            .entry(domain.to_string())
            .or_insert_with(|| vec![])
            .push(record.clone());

        Ok(record)
    }

    async fn delete_txt_record(&self, record_id: &str) -> Result<(), anyhow::Error> {
        for (_, v) in self.records.lock().await.iter_mut() {
            v.iter_mut()
                .position(|record| record.id == record_id)
                .map(|idx| v.remove(idx));
        }

        Ok(())
    }

    async fn list_txt_records(&self, domain: &str) -> Result<Vec<TxtRecord>, anyhow::Error> {
        Ok(self
            .records
            .lock()
            .await
            .get(domain)
            .unwrap_or(&vec![])
            .clone())
    }
}

pub struct Fixture {
    pub client: TestClient,
    pub state: AntZookeeperState,

    _guard: postgresql_embedded::PostgreSQL,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // let _ = remove_dir_all(&self.state.root_dir);
    }
}

#[derive(Clone)]
struct TestAntHostAgentService {
    _ant_host_agent_handle: Arc<JoinHandle<()>>,

    cfg: AntHostAgentClientConfig,
}

impl TestAntHostAgentService {
    pub async fn new(ant_host_agent_service: axum::Router) -> Result<Self, anyhow::Error> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            let server = axum::serve(listener, ant_host_agent_service);
            server.await.expect("server error");
        });

        let cfg = AntHostAgentClientConfig {
            endpoint: addr.ip().to_string(),
            port: addr.port(),
        };

        Ok(Self {
            _ant_host_agent_handle: Arc::new(handle),
            cfg,
        })
    }
}

impl AntHostAgentClientFactory for TestAntHostAgentService {
    fn new_client(&self, _cfg: AntHostAgentClientConfig) -> AntHostAgentClient {
        AntHostAgentClient::new(self.cfg.clone())
    }
}

impl Fixture {
    pub async fn new(function_name: &str) -> Self {
        let (_guard, test_db_config) = test_database_config("ant-zoo-storage").await;

        let root_dir = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("integration")
            .join("test-fs")
            .join(function_name);
        create_dir_all(&root_dir).await.unwrap();

        let ant_host_agent_state = AntHostAgentState {
            secrets_root_dir: root_dir.join("hostagent-secrets"),
            archive_root_dir: root_dir.join("hostagent-archive"),
            install_root_dir: root_dir.join("hostagent-install"),
        };
        create_dir_all(&ant_host_agent_state.secrets_root_dir)
            .await
            .unwrap();
        create_dir_all(&ant_host_agent_state.archive_root_dir)
            .await
            .unwrap();
        create_dir_all(&ant_host_agent_state.install_root_dir)
            .await
            .unwrap();

        let ant_host_agent_service = ant_host_agent::make_routes(ant_host_agent_state).unwrap();

        let state = AntZookeeperState {
            dns: Arc::new(Mutex::new(TestDns::new())),
            rng: OsRng,
            acme_url: acme_lib::DirectoryUrl::LetsEncryptStaging,
            acme_contact_email: "integ-test@typesofants.org".to_string(),
            root_dir: root_dir,
            db: AntZooStorageClient::connect(&test_db_config).await.unwrap(),
            ant_host_agent_factory: Arc::new(Mutex::new(
                TestAntHostAgentService::new(ant_host_agent_service)
                    .await
                    .unwrap(),
            )),
        };

        let routes = make_routes(state.clone()).unwrap();

        let client = TestClient::new(routes).await;

        Fixture {
            client,
            state,
            _guard,
        }
    }
}
