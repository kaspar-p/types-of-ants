use std::{collections::HashMap, path::PathBuf, sync::Arc};

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
use tokio::sync::Mutex;

struct TestDnsRecord {
    id: String,
    record_content: String,
}

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
    db: postgresql_embedded::PostgreSQL,
}

impl Fixture {
    pub async fn new() -> Self {
        let (db, test_db_config) = test_database_config("ant-zoo-storage").await;

        let state = AntZookeeperState {
            dns: Arc::new(Mutex::new(TestDns::new())),
            rng: OsRng,
            acme_url: acme_lib::DirectoryUrl::LetsEncryptStaging,
            acme_contact_email: "integ-test@typesofants.org".to_string(),
            root_dir: PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
                .join("tests")
                .join("integration")
                .join("fs"),
            db: Arc::new(Mutex::new(
                AntZooStorageClient::connect(&test_db_config).await.unwrap(),
            )),
        };

        let routes = make_routes(state.clone()).unwrap();

        let client = TestClient::new(routes).await;

        Fixture { client, state, db }
    }
}
