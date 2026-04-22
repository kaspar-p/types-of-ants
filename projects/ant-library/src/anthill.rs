use std::{fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthillManifest {
    // pub project: String,
    // pub build: String,
    pub deployment: Option<DeploymentOptions>,
    pub secrets: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentOptions {
    pub versioned: Option<bool>,
}

pub fn get_manifest_from_file(path: &Path) -> Result<AnthillManifest, anyhow::Error> {
    let mut manifest_buf = String::new();
    File::open(path)?.read_to_string(&mut manifest_buf)?;

    debug!("manifest: {}", manifest_buf);
    let manifest: AnthillManifest = serde_json::from_str(&manifest_buf)?;

    Ok(manifest)
}
