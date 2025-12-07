use std::{env::set_var, path::PathBuf};

use ant_zookeeper::routes::certs::ProvisionCertificateRequest;
use http::StatusCode;
use tracing_test::traced_test;

use crate::fixture::Fixture;

pub mod fixture;

#[traced_test]
#[tokio::test]
async fn test_lets_encrypt_staging_flow() {
    set_var(
        "TYPESOFANTS_SECRET_DIR",
        PathBuf::from(dotenv::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("..")
            .join("secrets")
            .join("dev"),
    );

    let fixture = Fixture::with_real_dns().await;
    {
        let req = ProvisionCertificateRequest {
            domains: vec!["example01.typesofants.org".to_string()],
        };
        let response = fixture.client.post("/certs/cert").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
