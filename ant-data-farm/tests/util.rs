use rstest::fixture;
use std::process::Command;
use testcontainers::{clients::Cli, images::generic::GenericImage};

#[fixture]
#[once]
pub fn logging() -> () {
    std::env::set_var("RUST_LOG", "ant_data_farm=debug,glimmer=debug");
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
}

pub struct TestFixture {
    pub docker: Cli,
    pub image: GenericImage,
}

#[must_use] pub fn test_fixture() -> TestFixture {
    let cwd: String = dotenv::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR present!");
    let db_name = dotenv::var("DB_PG_NAME").expect("DB_PG_NAME environment variable!");
    let user = dotenv::var("DB_PG_USER").expect("DB_PG_USER environment variable!");
    let pw = dotenv::var("DB_PG_PASSWORD").expect("DB_PG_PASSWORD environment variable!");

    // Build the test images in the repository
    let output = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(&format!("{cwd}/ant-data-farm.dockerfile"))
        .arg("--force-rm")
        // .arg(format!("-p {}:5432", db_port))
        .arg("--tag")
        .arg("ant-data-farm:latest")
        .arg(".")
        .output()
        .expect("Building the ant-data-farm docker image worked!");
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8(output.stderr).unwrap());
        panic!("Unable to build ant-data-farm:latest");
    }

    let image = GenericImage::new("ant-data-farm", "latest")
        .with_env_var("POSTGRES_DB", db_name)
        .with_env_var("POSTGRES_PASSWORD", pw)
        .with_env_var("POSTGRES_USER", user)
        .with_wait_for(testcontainers::core::WaitFor::StdOutMessage {
            message: "database system is ready to accept connections".to_owned(),
        });

    let docker = Cli::docker();

    TestFixture { docker, image }
}
