use std::{
    env::set_var,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    sync::Arc,
};

use ant_archive::{AntArchiveState, AntArchiveStorageNodeClient, make_routes};
use ant_archive_db::AntArchiveDb;
use ant_archive_storage::{AntArchiveStorageState, build_metric_layer, make_routes as make_storage_routes};
use ant_library::db::{DatabaseConfig, TypesOfAntsDatabase};
use ant_library_test::{axum_test_client::TestClient, db::TestDatabase};
use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha256};
use tokio::{net::TcpListener, task::JoinHandle};

pub const TEST_BEARER_TOKEN: &str = "test-bearer-token-for-ant-archive";
pub const TEST_NODE_ID: &str = "sn-testnode";
const TEST_KEK_ID: &str = "kek-test";
const TEST_CLIENT_ID: &str = "client-test";
pub const TEST_BUCKET_ID: &str = "b-testbucket";
pub const TEST_PUBLIC_BUCKET_ID: &str = "b-testpublic";
pub const TEST_INTERNAL_BUCKET_ID: &str = "b-testinternal";

pub struct StorageNode {
    pub node_id: String,
    pub base_url: String,
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
        let addr = listener.local_addr().unwrap();

        let (metric_layer, handle) = build_metric_layer();
        let state = AntArchiveStorageState::new(root.clone(), handle);
        let app = make_storage_routes(state, metric_layer).unwrap();

        let join = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("storage server error");
        });

        StorageNode {
            node_id: TEST_NODE_ID.to_string(),
            base_url: format!("http://{}", addr),
            root,
            _handle: Arc::new(join),
        }
    }

    pub fn client(&self) -> AntArchiveStorageNodeClient {
        AntArchiveStorageNodeClient::new(
            self.node_id.clone(),
            self.base_url.clone(),
            "user",
            "test-password",
        )
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

        let db = TestDatabase::new("ant-archive-db").await;
        let storage = StorageNode::new(name).await;

        let archive_db = AntArchiveDb::connect(&db.config).await.unwrap();
        seed_db(&archive_db).await;

        let kek: [u8; 32] = hex::decode(
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        )
        .unwrap()
        .try_into()
        .unwrap();

        let state = AntArchiveState::new(
            archive_db,
            vec![storage.client()],
            TEST_KEK_ID.to_string(),
            kek,
        );

        let app = make_routes(state).unwrap();

        Fixture {
            client: TestClient::new(app).await,
            bearer_token: TEST_BEARER_TOKEN.to_string(),
            bucket_id: TEST_BUCKET_ID.to_string(),
            public_bucket_id: TEST_PUBLIC_BUCKET_ID.to_string(),
            internal_bucket_id: TEST_INTERNAL_BUCKET_ID.to_string(),
            _db: db,
            _storage: storage,
        }
    }
}

async fn seed_db(db: &AntArchiveDb) {
    let token_hash = Base64::encode_string(&Sha256::digest(TEST_BEARER_TOKEN.as_bytes()));

    db.register_kek(TEST_KEK_ID).await.unwrap();
    db.register_storage_node(TEST_NODE_ID, "test-node")
        .await
        .unwrap();
    db.create_client(TEST_CLIENT_ID, "test-client", &token_hash)
        .await
        .unwrap();
    db.create_bucket(TEST_BUCKET_ID, TEST_CLIENT_ID, true, "private")
        .await
        .unwrap();
    db.create_bucket(TEST_PUBLIC_BUCKET_ID, TEST_CLIENT_ID, false, "public")
        .await
        .unwrap();
    db.create_bucket(TEST_INTERNAL_BUCKET_ID, TEST_CLIENT_ID, false, "internal")
        .await
        .unwrap();
}
