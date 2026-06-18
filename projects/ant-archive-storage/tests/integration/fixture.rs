use std::{
    env::set_var,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key,
};
use ant_archive_storage::{build_metric_layer, make_routes, AntArchiveStorageState};
use ant_library_test::axum_test_client::TestClient;

const TEST_TEK: [u8; 32] = [42u8; 32];

pub struct TestFixture {
    pub root: PathBuf,
    pub client: TestClient,
    pub state: AntArchiveStorageState,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        remove_dir_all(self.root.clone()).unwrap();
    }
}

impl TestFixture {
    /// Wraps `content` as the outer blob that the ant-archive router sends to storage.
    ///
    /// Mirrors the router's upload path: produces a fake inner (nonce || content || tag,
    /// length >= 28) then AES-GCM-encrypts it with TEST_TEK to form the outer blob.
    ///
    /// Returns (outer_bytes, "X-Ant-Tek" header value).
    pub fn make_outer_blob(&self, content: &[u8]) -> (Vec<u8>, String) {
        // Fake inner: 12-byte nonce prefix + content + 16-byte tag suffix.
        // Guarantees len(inner) = 28 + content.len() >= 28.
        let mut inner = vec![0u8; 12];
        inner.extend_from_slice(content);
        inner.extend_from_slice(&[0u8; 16]);

        let key = Key::<Aes256Gcm>::from_slice(&TEST_TEK);
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, inner.as_slice()).unwrap();

        let mut outer = nonce.to_vec();
        outer.extend(ciphertext);

        let tek_header = base16ct::lower::encode_string(&TEST_TEK);
        (outer, tek_header)
    }
}

pub async fn test_router_no_auth(name: &str) -> TestFixture {
    unsafe {
        set_var(
            "TYPESOFANTS_SECRET_DIR",
            PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
                .join("tests")
                .join("integration")
                .join("test-secrets")
                .to_str()
                .unwrap()
                .to_string(),
        );
    }

    let root = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests")
        .join("integration")
        .join("test-blobs")
        .join(name);
    create_dir_all(&root).unwrap();

    let (metric_layer, handle) = build_metric_layer();
    let state = AntArchiveStorageState::new(root.clone(), handle);
    let api = make_routes(state.clone(), metric_layer).unwrap();

    TestFixture {
        client: TestClient::new(api).await,
        state,
        root,
    }
}

pub async fn test_router_auth(name: &str) -> (TestFixture, String) {
    let fixture = test_router_no_auth(name).await;
    // "user:test-password" base64-encoded for Basic auth
    (fixture, "Basic dXNlcjp0ZXN0LXBhc3N3b3Jk".to_string())
}
