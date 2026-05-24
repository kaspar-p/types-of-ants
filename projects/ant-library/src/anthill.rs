use std::{fs::File, io::Read, path::Path, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthillManifest {
    pub project: String,
    pub build: AnthillBuild,
    #[serde(default)]
    pub build_parallelism: AnthillBuildParallelism,
    pub archetype: Option<AnthillArchetype>,
    pub deployment: Option<DeploymentOptions>,
    pub secrets: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnthillBuildParallelism {
    #[serde(rename = "serial")]
    Serial,

    #[serde(rename = "parallel")]
    Parallel,
}

impl Default for AnthillBuildParallelism {
    fn default() -> Self {
        Self::Parallel
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnthillBuild {
    #[serde(rename = "makefile")]
    Makefile,

    #[serde(rename = "docker")]
    Docker,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnthillArchetype {
    #[serde(rename = "postgres")]
    Postgres,

    #[serde(rename = "webservice")]
    Webservice,
}

impl FromStr for AnthillArchetype {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" => Ok(AnthillArchetype::Postgres),
            "webservice" => Ok(AnthillArchetype::Webservice),
            s => Err(format!("No such archetype: {s}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentOptions {
    /// The main port meant for this project, for service discovery.
    pub port: Option<u16>,

    /// A different port for discovering metrics, e.g. for webservers that don't want to expose their metrics to the world.
    pub metrics_port: Option<u16>,

    pub versioned: Option<bool>,
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

        let manifest: AnthillManifest = serde_json::from_str(&manifest_buf)?;

        Ok(manifest)
    }
}
