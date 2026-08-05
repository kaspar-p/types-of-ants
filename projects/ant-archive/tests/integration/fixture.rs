use std::{
    collections::HashMap,
    env::set_var,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    sync::Arc,
};

use serde::Deserialize;

use ant_archive::{make_routes, AntArchiveDb, AntArchiveState};
use ant_archive_db::ClientCapabilities;
use ant_archive_storage::{
    build_metric_layer, make_routes as make_storage_routes, AntArchiveStorageState,
};
use ant_library::{
    db::TypesOfAntsDatabase as _,
    rng::TestSeededRng,
    sd::{reader::ServiceDiscovery, writer::ServiceDiscoveryWriter},
};
use ant_library_test::{
    axum_test_client::TestClient, consul_fixture::ConsulFixture, db::TestDatabase,
};
use tokio::{net::TcpListener, task::JoinHandle};

pub const TEST_BEARER_TOKEN: &str = "test-bearer-token-for-ant-archive";
const TEST_BUCKET_ID: &str = "b-testbucket";
const TEST_PUBLIC_BUCKET_ID: &str = "b-testpublic";
const TEST_INTERNAL_BUCKET_ID: &str = "b-testinternal";

pub struct BucketIds {
    pub private_id: String,
    pub public_id: String,
    pub internal_id: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Visibility {
    Public,
    Internal,
    Private,
}

#[derive(Deserialize)]
struct Bucket {
    bucket_id: String,
    visibility: Visibility,
}

#[derive(Deserialize)]
struct BucketList {
    buckets: Vec<Bucket>,
}

pub struct StorageNode {
    pub port: u16,
    root: PathBuf,
    _handle: Arc<JoinHandle<()>>,
}

impl Drop for StorageNode {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.root);
    }
}

impl StorageNode {
    pub async fn new(name: &str, bind_host: &str) -> Self {
        let root = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("integration")
            .join("test-blobs")
            .join(name);
        create_dir_all(&root).unwrap();

        let addr = format!("{bind_host}:0");
        let listener = TcpListener::bind(&addr)
            .await
            .expect(&format!("could not bind ephemeral storage socket: {addr}"));
        let port = listener.local_addr().unwrap().port();

        let (metric_layer, handle) = build_metric_layer();
        let state = AntArchiveStorageState::new(root.clone(), handle);
        let app = make_storage_routes(state, metric_layer).unwrap();

        let join = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("storage server error");
        });

        StorageNode {
            port,
            root,
            _handle: Arc::new(join),
        }
    }
}

pub struct Fixture {
    pub client: TestClient,
    pub bearer_token: String,
    pub db: AntArchiveDb,
    pub sd: Arc<ServiceDiscovery>,
    pub consul_port: u16,
    _db: TestDatabase,
    _storages: Vec<StorageNode>,
    _consul: ConsulFixture,
}

impl Fixture {
    pub async fn new(name: &str) -> Self {
        Self::new_with_capacity(name, 1024 * 1024 * 1024).await
    }

    pub async fn bucket_ids(&self) -> BucketIds {
        let body: BucketList = self
            .client
            .get("/buckets")
            .header("Authorization", &format!("Bearer {}", self.bearer_token))
            .send()
            .await
            .json()
            .await;

        let find = |v: Visibility| {
            body.buckets
                .iter()
                .find(|b| b.visibility == v)
                .map(|b| b.bucket_id.clone())
                .unwrap_or_else(|| panic!("no {:?} bucket found", stringify!(v)))
        };

        BucketIds {
            private_id: find(Visibility::Private),
            public_id: find(Visibility::Public),
            internal_id: find(Visibility::Internal),
        }
    }

    pub async fn new_with_capacities(name: &str, capacities: HashMap<String, i64>) -> Self {
        unsafe {
            set_var(
                "TYPESOFANTS_SECRET_DIR",
                PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
                    .join("tests")
                    .join("integration")
                    .join("test-secrets"),
            );
        }

        let consul = ConsulFixture::new().await;
        let db = TestDatabase::new("ant-archive-db").await;

        let mut storages = vec![];
        for (suffix, host) in [
            // Hosts need to match the credentials in the test secrets file
            ("sn1", "127.0.0.1"),
            ("sn2", "127.0.0.1"),
            ("sn3", "127.0.0.1"),
        ] {
            let node_name = format!("{name}-{suffix}");
            let sn = StorageNode::new(&node_name, host).await;
            ServiceDiscoveryWriter::new(consul.port())
                .register_remote_service("ant-archive-storage", &suffix, host, sn.port)
                .await
                .expect("failed to register storage node with Consul");
            storages.push(sn);
        }

        let storages2 = ServiceDiscovery::new(consul.port())
            .resolve_all("ant-archive-storage")
            .await;
        println!("ALL STORAGES: {:?}", storages2);

        // Register the storage node with the test Consul instance.

        let archive_db = AntArchiveDb::connect(&db.config).await.unwrap();
        seed_db(&archive_db, capacities).await;

        let sd = Arc::new(ServiceDiscovery::new(consul.port()));
        let state = AntArchiveState {
            chunk_size: 10, // 10 bytes
            db: archive_db.clone(),
            sd: sd.clone(),
            rng: Arc::new(TestSeededRng::new(42)),
        };
        let app = make_routes(state);

        Fixture {
            client: TestClient::new(app).await,
            bearer_token: TEST_BEARER_TOKEN.to_string(),
            db: archive_db,
            sd,
            consul_port: consul.port(),
            _db: db,
            _storages: storages,
            _consul: consul,
        }
    }

    pub async fn new_with_capacity(name: &str, capacity_bytes: i64) -> Self {
        let mut map = HashMap::new();
        map.insert("sn-test1".to_string(), capacity_bytes);
        map.insert("sn-test2".to_string(), capacity_bytes);
        map.insert("sn-test3".to_string(), capacity_bytes);

        Self::new_with_capacities(name, map).await
    }
}

async fn seed_db(db: &AntArchiveDb, capacities: HashMap<String, i64>) {
    db.register_kek("default").await.unwrap();

    // host_id matches the Consul node name (in test_secrets) so resolve_storage_nodes can find it.
    db.register_storage_node(
        "sn-test1",
        "sn1",
        *capacities.get("sn-test1").unwrap_or(&0),
        "http",
    )
    .await
    .unwrap();
    db.register_storage_node(
        "sn-test2",
        "sn2",
        *capacities.get("sn-test2").unwrap_or(&0),
        "http",
    )
    .await
    .unwrap();
    db.register_storage_node(
        "sn-test3",
        "sn3",
        *capacities.get("sn-test2").unwrap_or(&0),
        "http",
    )
    .await
    .unwrap();

    let client_id = db
        .create_client("test-client", &TEST_BEARER_TOKEN)
        .await
        .unwrap();
    db.set_client_capabilities(
        &client_id,
        &ClientCapabilities {
            can_select_storage_node: true,
        },
    )
    .await
    .unwrap();

    db.create_bucket(TEST_BUCKET_ID, &client_id, true, "private")
        .await
        .unwrap();
    db.create_bucket(TEST_PUBLIC_BUCKET_ID, &client_id, false, "public")
        .await
        .unwrap();
    db.create_bucket(TEST_INTERNAL_BUCKET_ID, &client_id, false, "internal")
        .await
        .unwrap();
}
