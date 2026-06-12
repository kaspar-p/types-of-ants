use std::{
    env::set_var,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use ant_archive_storage::{build_metric_layer, make_routes, AntArchiveStorageState};
use ant_library_test::axum_test_client::TestClient;

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
