use std::{
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
const TEST_KEK_ID: &str = "kek-test";
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
    pub db: AntArchiveDb,
    pub sd: Arc<ServiceDiscovery>,
    pub consul_port: u16,
    _db: TestDatabase,
    _storage: StorageNode,
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

    pub async fn new_with_capacity(name: &str, capacity_bytes: i64) -> Self {
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
        seed_db(&archive_db, capacity_bytes).await;

        let sd = Arc::new(ServiceDiscovery::new(consul.port()));
        let state = AntArchiveState {
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
            _storage: storage,
            _consul: consul,
        }
    }
}

async fn seed_db(db: &AntArchiveDb, capacity_bytes: i64) {
    db.register_kek(TEST_KEK_ID).await.unwrap();

    // host_id matches the Consul node name so resolve_storage_nodes can find it.
    db.register_storage_node("sn-test", CONSUL_NODE_NAME, capacity_bytes, "http")
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
