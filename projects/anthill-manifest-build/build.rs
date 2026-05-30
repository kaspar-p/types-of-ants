// build.rs
use schemars::schema_for;
use std::path::PathBuf;

use anthill_manifest::AnthillManifest;

fn main() {
    println!("cargo:rerun-if-changed=../anthill-manifest/src/lib.rs");

    // Generate the JSON Schema object
    let schema = schema_for!(AnthillManifest);

    // Stringify it into pretty-printed JSON
    let schema_json = serde_json::to_string_pretty(&schema).unwrap();

    // Define your output path (e.g., project root)
    let dest_path = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap())
        .join("schemas")
        .join("anthill_schema.json");

    std::fs::create_dir_all(dest_path.parent().unwrap()).unwrap();
    std::fs::write(dest_path, schema_json.as_bytes()).unwrap();
}
