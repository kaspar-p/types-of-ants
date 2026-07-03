use std::path::PathBuf;

use tokio::test;
use tracing_test::traced_test;

fn setup() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests")
        .join("integration")
        .join("test-secrets");
    unsafe {
        std::env::set_var("TYPESOFANTS_SECRET_DIR", dir);
    }
}

#[test]
#[traced_test]
async fn reads_from_no_extension() {
    setup();

    let content = ant_library::secret::load_secret("secret-no-extension").unwrap();
    assert_eq!(content, "secret-content-here-no-extension");
}

#[test]
#[traced_test]
async fn reads_from_extension() {
    setup();

    let content = ant_library::secret::load_secret("secret-extension").unwrap();
    assert_eq!(content, "secret-content-here-from-extension");
}

#[test]
#[traced_test]
async fn prefers_extension_over_without() {
    setup();

    let content = ant_library::secret::load_secret("secret-both").unwrap();
    assert_eq!(content, "content1");
}
