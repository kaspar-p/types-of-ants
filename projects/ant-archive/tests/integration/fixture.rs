use std::{
    env::set_var,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    sync::Arc,
};

use ant_archive::{make_routes, AntArchiveDb, AntArchiveState};
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
const TEST_KEK_ID: &str = "kek-test";
pub const TEST_BUCKET_ID: &str = "b-testbucket";
pub const TEST_PUBLIC_BUCKET_ID: &str = "b-testpublic";
pub const TEST_INTERNAL_BUCKET_ID: &str = "b-testinternal";

// The Consul node name used by ConsulFixture.
const CONSUL_NODE_NAME: &str = "test-node1";

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
    pub async fn new(name: &str) -> Self {
        let root = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("integration")
            .join("test-blobs")
            .join(name);
        create_dir_all(&root).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("could not bind ephemeral storage socket");
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
    pub bucket_id: String,
    pub public_bucket_id: String,
    pub internal_bucket_id: String,
    _db: TestDatabase,
    _storage: StorageNode,
    _consul: ConsulFixture,
}

impl Fixture {
    pub async fn new(name: &str) -> Self {
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
        let storage = StorageNode::new(name).await;

        // Register the storage node with the test Consul instance.
        ServiceDiscoveryWriter::new(consul.port())
            .register_remote_service("ant-archive-storage", "127.0.0.1", storage.port)
            .await
            .expect("failed to register storage node with Consul");

        let archive_db = AntArchiveDb::connect(&db.config).await.unwrap();
        seed_db(&archive_db).await;

        let sd = Arc::new(ServiceDiscovery::new(consul.port()));
        let state = AntArchiveState { db: archive_db, sd, rng: Arc::new(TestSeededRng::new(42)) };
        let app = make_routes(state);

        Fixture {
            client: TestClient::new(app).await,
            bearer_token: TEST_BEARER_TOKEN.to_string(),
            bucket_id: TEST_BUCKET_ID.to_string(),
            public_bucket_id: TEST_PUBLIC_BUCKET_ID.to_string(),
            internal_bucket_id: TEST_INTERNAL_BUCKET_ID.to_string(),
            _db: db,
            _storage: storage,
            _consul: consul,
        }
    }
}

async fn seed_db(db: &AntArchiveDb) {
    db.register_kek(TEST_KEK_ID).await.unwrap();

    // host_id matches the Consul node name so resolve_storage_nodes can find it.
    db.register_storage_node("sn-test", CONSUL_NODE_NAME)
        .await
        .unwrap();
    let client_id = db
        .create_client("test-client", &TEST_BEARER_TOKEN)
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
