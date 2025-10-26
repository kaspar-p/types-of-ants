use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use ant_host_agent::{make_routes, state::AntHostAgentState};
use ant_library::axum_test_client::TestClient;

pub struct TestFixture {
    test_root_dir: PathBuf,
    pub client: TestClient,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        remove_dir_all(self.test_root_dir.clone()).unwrap();
    }
}

impl TestFixture {
    pub async fn new(name: &str) -> Self {
        let test_root_dir = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("test-fs")
            .join(name);
        let _ = remove_dir_all(test_root_dir.clone());

        let archive_root_dir = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("integration")
            .join("archives");
        create_dir_all(&archive_root_dir).unwrap();

        let install_root_dir = test_root_dir.join("service");
        create_dir_all(&install_root_dir).unwrap();

        let state = AntHostAgentState {
            archive_root_dir: archive_root_dir.clone(),
            install_root_dir: install_root_dir.clone(),
        };

        let client = TestClient::new(make_routes(state.clone()).await.unwrap()).await;

        TestFixture {
            client,
            test_root_dir,
        }
    }
}
