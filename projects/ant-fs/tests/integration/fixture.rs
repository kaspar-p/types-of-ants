use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use ant_fs::make_routes;
use ant_library::axum_test_client::TestClient;

pub struct TestFixture {
    root: PathBuf,
    pub client: TestClient,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        remove_dir_all(self.root.clone()).unwrap();
    }
}

pub async fn test_router_no_auth(name: &str) -> TestFixture {
    let root = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
        .join("test-fs")
        .join(name);
    create_dir_all(&root).unwrap();
    let api = make_routes(root.clone()).unwrap();

    TestFixture {
        client: TestClient::new(api).await,
        root,
    }
}

pub async fn test_router_auth(name: &str) -> (TestFixture, String) {
    let fixture = test_router_no_auth(name).await;

    (fixture, "Basic dXNlcjp0ZXN0LXBhc3N3b3Jk".to_string()) // user:test-password
}
