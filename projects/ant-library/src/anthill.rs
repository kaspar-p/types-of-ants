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

    /// Optionally deploy more than 1 systemd services alongside this one, with other entrypoints.
    /// All services get the same persist directory, working directory, secrets directory, and secrets.
    pub services: Vec<AnthillService>,

    /// A map of port-identifier to port number, for example:
    /// ```json
    /// {
    ///   "primary": 3000
    /// }
    /// ```
    /// where the port identifier will become a key in ant-matchmaker's Consul "meta" tag.
    ///
    /// Note that primary is the main port, but some projects that don't have main APIs
    /// may only have the config port, or only have a metrics port.
    pub ports: Option<Ports>,

    /// The secrets that will be exposed to this project at deploy-time, by name.
    /// The project can expect that {installation-dir}/secrets/{name}.secret will exist
    /// if configured as:
    /// ```json
    /// {
    ///   "secrets": ["name"]
    /// }
    /// ```
    #[serde(default)]
    pub secrets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthillService {
    /// The name of the service. If not specified
    pub service: String,

    /// The entrypoint script name. Must be located at .anthill/{entrypoint}
    /// For example, "run.sh" must be a file at ".anthill/run.sh"
    pub entrypoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ports {
    pub primary: Option<u16>,
    pub metrics: Option<u16>,
    pub config: Option<u16>,
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
    #[deprecated(note = "Prefer .ports.primary")]
    pub port: Option<u16>,

    /// A different port for discovering metrics, e.g. for webservers that don't want to expose their metrics to the world.
    #[deprecated(note = "Prefer .ports.metrics")]
    pub metrics_port: Option<u16>,

    #[deprecated(
        note = "All projects now support unversioned deployments via the 'current' symlink"
    )]
    pub versioned: Option<bool>,
}

#[derive(thiserror::Error, Debug)]
pub enum AnthillManifestError {
    #[error("failed to read anthill manifest")]
    Io(#[from] std::io::Error),

    #[error("malformed anthill manifest")]
    Shape(#[from] serde_json::Error),

    #[error("malformed anthill manifest")]
    Malformed(#[from] anyhow::Error),
}

impl AnthillManifest {
    pub fn from_file(path: &Path) -> Result<Self, AnthillManifestError> {
        let mut manifest_buf = String::new();
        File::open(path)?.read_to_string(&mut manifest_buf)?;

        let manifest: AnthillManifest = serde_json::from_str(&manifest_buf)?;
        manifest.validate()?;

        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        // The services array, if contains other services like:
        //      service: "ant-lumberjack-config"
        // then must also contain:
        //      service: "ant-lumberjack"
        // to not cause confusion.
        if self.services.len() > 0
            && self
                .services
                .iter()
                .find(|s| s.service == self.project)
                .is_none()
        {
            return Err(anyhow::Error::msg(format!(
                "The .services entry is nonempty but does not contain entry with 'service={}'",
                self.project
            )));
        }

        Ok(())
    }
}
