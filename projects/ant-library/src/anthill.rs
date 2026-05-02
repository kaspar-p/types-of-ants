use std::{fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthillManifest {
    // pub project: String,
    pub build: AnthillBuild,
    pub deployment: Option<DeploymentOptions>,
    pub secrets: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnthillBuild {
    #[serde(rename = "makefile")]
    Makefile,

    #[serde(rename = "docker")]
    Docker,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentOptions {
    pub versioned: Option<bool>,
}

impl Default for DeploymentOptions {
    fn default() -> Self {
        Self {
            versioned: Some(true),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AnthillManifestError {
    #[error("failed to read anthill manifest")]
    Io(#[from] std::io::Error),

    #[error("malformed anthill manifest")]
    Shape(#[from] serde_json::Error),
}

impl AnthillManifest {
    pub fn from_file(path: &Path) -> Result<Self, AnthillManifestError> {
        let mut manifest_buf = String::new();
        File::open(path)?.read_to_string(&mut manifest_buf)?;

        debug!("manifest: {}", manifest_buf);
        let manifest: AnthillManifest = serde_json::from_str(&manifest_buf)?;

        Ok(manifest)
    }
}
