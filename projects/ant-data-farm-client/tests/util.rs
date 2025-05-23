use rstest::fixture;
use std::process::Command;
use testcontainers::{
    core::client::docker_client_instance, ContainerRequest, GenericImage, ImageExt,
};
use tracing::debug;

#[fixture]
#[once]
pub fn logging() -> () {
    std::env::set_var("RUST_LOG", "ant_data_farm=debug,glimmer=debug");
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
}

pub struct TestFixture {
    pub image: ContainerRequest<GenericImage>,
}

#[must_use]
pub async fn test_fixture(tag: &str) -> TestFixture {
    let cwd: String = dotenv::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR present!");
    println!("{}", cwd);
    let db_name = dotenv::var("DB_PG_NAME").expect("DB_PG_NAME environment variable!");
    let user = dotenv::var("DB_PG_USER").expect("DB_PG_USER environment variable!");
    let pw = dotenv::var("DB_PG_PASSWORD").expect("DB_PG_PASSWORD environment variable!");

    // Build the test images in the repository
    let output = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(&format!("{cwd}/../ant-data-farm/Dockerfile"))
        .arg("--force-rm")
        .arg("--tag")
        .arg(format!("ant-data-farm:{}", tag))
        .arg(&format!("{cwd}/../ant-data-farm"))
        .output()
        .expect("Building the ant-data-farm docker image worked!");
    debug!("Built docker image!");
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8(output.stderr).unwrap());
        panic!("Unable to build ant-data-farm:{}", tag);
    }

    let image = GenericImage::new("ant-data-farm", tag)
        .with_wait_for(testcontainers::core::WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("PGDATA", "/var/lib/postgresql/data")
        .with_env_var("POSTGRES_DB", db_name)
        .with_env_var("POSTGRES_PASSWORD", pw)
        .with_env_var("POSTGRES_USER", user);

    let docker = docker_client_instance().await.unwrap();

    TestFixture { image }
}
