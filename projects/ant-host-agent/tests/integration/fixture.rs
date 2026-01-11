use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use ant_host_agent::{make_routes, routes::secret::PutSecretRequest, state::AntHostAgentState};
use ant_library_test::axum_test_client::TestClient;
use hyper::StatusCode;

pub struct TestFixture {
    pub test_root_dir: PathBuf,
    pub client: TestClient,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        remove_dir_all(self.test_root_dir.clone()).unwrap();
    }
}

impl TestFixture {
    pub async fn new(name: &str, use_ephemeral_archive_dir: Option<bool>) -> Self {
        let test_root_dir = PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("test-fs")
            .join(name);
        let _ = remove_dir_all(test_root_dir.clone());

        let archive_root_dir = match use_ephemeral_archive_dir {
            Some(true) => test_root_dir.join("fs"),
            Some(false) | None => PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
                .join("tests")
                .join("integration")
                .join("archives"),
        };
        create_dir_all(&archive_root_dir).unwrap();

        let test_secrets_dir = test_root_dir.join("secrets");
        create_dir_all(&test_secrets_dir).unwrap();

        let install_root_dir = test_root_dir.join("service");
        create_dir_all(&install_root_dir).unwrap();

        let state = AntHostAgentState {
            archive_root_dir: archive_root_dir.clone(),
            install_root_dir: install_root_dir.clone(),
            secrets_root_dir: test_secrets_dir.clone(),
        };

        let client = TestClient::new(make_routes(state.clone()).unwrap()).await;

        {
            let req = PutSecretRequest {
                name: "test-secret1".to_string(),
                value: "secret value 1".as_bytes().to_vec(),
            };

            let response = client.post("/secret/secret").json(&req).send().await;
            assert_eq!(response.status(), StatusCode::OK);
        }

        {
            let req = PutSecretRequest {
                name: "test-secret2".to_string(),
                value: "secret value 2".as_bytes().to_vec(),
            };

            let response = client.post("/secret/secret").json(&req).send().await;
            assert_eq!(response.status(), StatusCode::OK);
        }

        TestFixture {
            client,
            test_root_dir,
        }
    }
}
