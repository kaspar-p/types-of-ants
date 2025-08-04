#[derive(serde::Serialize, serde::Deserialize)]
pub struct ManifestFile {
    pub project: String,
    pub project_type: String,
    pub version: String,
    pub commit_sha: String,
    pub commit_number: String,
    pub installed_at: String,
    pub unit_file: String,
}

/// Read a local ./manifest.json file as all rust binaries have near them.
pub fn read_local_manifest_file(dir: Option<&std::path::PathBuf>) -> ManifestFile {
    let mut path = std::path::PathBuf::new();
    if let Some(dir) = dir {
        path = path.join(dir);
    }
    path = path.join("manifest.json");

    let file = std::fs::File::open(path).expect("no ./manifest.json file");
    let file: ManifestFile =
        serde_json::from_reader(file).expect("local manifest.json file not json");

    file
}
