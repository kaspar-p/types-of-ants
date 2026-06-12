use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    sync::Arc,
};

use ant_host_agent::{make_routes, state::AntHostAgentState};
use ant_library::sd::writer::ServiceDiscoveryWriter;
use ant_library_test::{axum_test_client::TestClient, consul_fixture::ConsulFixture};
use flate2::{write::GzEncoder, Compression};
use tempfile::NamedTempFile;

pub struct TestFixture {
    archive_root_dir: PathBuf,
    consul: ConsulFixture,

    pub test_root_dir: PathBuf,
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

        let archive_root_dir = test_root_dir.join("fs");
        create_dir_all(&archive_root_dir).unwrap();

        let install_root_dir = test_root_dir.join("service");
        create_dir_all(&install_root_dir).unwrap();

        let consul = ConsulFixture::new().await;

        let state = AntHostAgentState {
            sd: Arc::new(ServiceDiscoveryWriter::new(consul.port())),
            archive_root_dir: archive_root_dir.clone(),
            install_root_dir: install_root_dir.clone(),
        };

        let client = TestClient::new(make_routes(state.clone()).unwrap()).await;

        TestFixture {
            client,
            consul,
            test_root_dir,
            archive_root_dir,
        }
    }

    /// Create a tarfile from a test directory in the "archives/" directory
    pub fn make_tarfile_fixture(&self, tar_dir_name: &str) -> NamedTempFile {
        std::fs::create_dir_all(self.archive_root_dir.join("tmp")).unwrap();
        let dst = NamedTempFile::new_in(self.archive_root_dir.join("tmp")).unwrap();

        let enc_dst = GzEncoder::new(&dst, Compression::best());

        {
            let mut tarfile = tar::Builder::new(enc_dst);

            tarfile
                .append_dir_all(
                    ".",
                    PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
                        .join("tests")
                        .join("integration")
                        .join("archives")
                        .join(tar_dir_name),
                )
                .unwrap();

            tarfile.finish().unwrap();
        }

        dst
    }
}
