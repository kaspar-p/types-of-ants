use std::{collections::HashMap, fs::File, io::Read, path::Path, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::AnthillSecret::{Expanded, Simple};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnthillManifest {
    pub project: String,
    pub description: Option<String>,

    pub build: AnthillBuild,

    #[serde(default)]
    pub build_parallelism: AnthillBuildParallelism,

    pub archetype: Option<AnthillArchetype>,

    pub deployment: Option<DeploymentOptions>,

    /// For the reverse-proxy to the outside, this setting affects the NGINX settings when
    /// this project deploys.
    #[serde(default)]
    pub routing: OneOrMany<Route>,

    /// Optionally deploy more than 1 systemd services alongside this one, with other entrypoints.
    /// All services get the same persist directory, working directory, secrets directory, and secrets.
    #[serde(default)]
    pub services: Vec<AnthillService>,

    /// A map of port-identifier to port number, for example:
    /// ```json
    /// { "primary": 3000 }
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
    /// { "secrets": ["name"] }
    /// ```
    #[serde(default)]
    pub secrets: Vec<AnthillSecret>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AnthillSecret {
    Simple(String),
    Expanded {
        name: String,
        host_specific: Option<bool>,
    },
}

impl AnthillSecret {
    pub fn name(&self) -> &str {
        match self {
            Simple(name) => name,
            Expanded { name, .. } => name,
        }
    }

    pub fn is_host_specific(&self) -> bool {
        match self {
            Simple(_) => false,
            Expanded { host_specific, .. } => host_specific.unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> Default for OneOrMany<T> {
    fn default() -> Self {
        Self::Many(vec![])
    }
}

// Convert to &[T] easily
impl<T> std::ops::Deref for OneOrMany<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            OneOrMany::One(item) => std::slice::from_ref(item),
            OneOrMany::Many(list) => list,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Route {
    /// If this project deploys a custom subdomain like "new-project.typesofants.org",
    /// then this should the full domain "new-project.typesofants.org".
    ///
    /// If the project just defines a subpath of "typesofants.org" it should still specify that
    /// alongside paths.
    pub domain: Option<String>,

    /// The query paths that get routed to this project. For example, ant-on-the-web wants
    /// all of the routes like /api/*, so would configure:
    /// ```json
    /// {
    ///   "paths": ["/api"]
    /// }
    /// ```
    ///
    /// If specified, domain must also be specified.
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnthillService {
    /// The name of the service. If not specified
    pub service: String,

    /// The entrypoint script name. Must be located at .anthill/{entrypoint}
    /// For example, "run.sh" must be a file at ".anthill/run.sh"
    pub entrypoint: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Ports {
    /// This is the port that is returned on service-discovery requests as the default port.
    /// Most services that expose some networked interface should specify this:
    /// - Web services should specify their main API port.
    /// - Databases should specify their server connection port.
    pub primary: Option<u16>,

    /// The port that metrics are served from. This MAY be the same port as the primary,
    /// in the case of databases for example where the metric are collected via regular
    /// connection and querying of database-specific tables.
    ///
    /// But for web-services or public services it wouldn't be safe to serve metrics from
    /// the same port that serves end-user traffic.
    ///
    /// If this setting is not set, the ant-monitor prometheus will have nothing to monitor.
    ///
    /// If for some reason the project cannot natively export metrics (e.g. nginx), this
    /// port may be missing.
    pub metrics: Option<Metrics>,

    /// A separate port for admin/control-plane functionality to affect the service at runtime.
    /// Generally something that could be done via deployment.
    ///
    /// For example, the ant-lumberjack promtail project needs a dynamic set of log => metric
    /// rules to emit for ant-monitor. These dynamic rules are set at deploy-time OF OTHER SERVICES.
    /// There is a small server listening for those updates at this "config" port.
    ///
    /// The deployment system will use this port for deploy-time dynamic actions.
    pub config: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum AnthillBuildParallelism {
    #[serde(rename = "serial")]
    Serial,

    #[serde(rename = "parallel")]
    Parallel,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Metrics {
    Port(u16),
    PortAndPath { port: u16, path: String },
}

impl Metrics {
    pub fn port(&self) -> u16 {
        match self {
            Metrics::PortAndPath { port, .. } => *port,
            Metrics::Port(port) => *port,
        }
    }
}

impl Default for AnthillBuildParallelism {
    fn default() -> Self {
        Self::Parallel
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum AnthillBuild {
    #[serde(rename = "makefile")]
    Makefile,

    #[serde(rename = "docker")]
    Docker,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeploymentOptions {
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

    /// Returns port-derived environment variables for the given project name.
    /// Produces both short names (PORT, METRICS_PORT, CONFIG_PORT) and
    /// project-scoped names (ANT_FOO_PORT, ANT_FOO_METRICS_PORT, ANT_FOO_CONFIG_PORT).
    pub fn to_port_env_vars(&self, project: &str) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        let upcase = project.to_uppercase().replace('-', "_");

        let primary = self.ports.as_ref().and_then(|p| p.primary);
        if let Some(port) = primary {
            vars.insert("PORT".to_string(), port.to_string());
            vars.insert("PRIMARY_PORT".to_string(), port.to_string());
            vars.insert(format!("{upcase}_PORT"), port.to_string());
            vars.insert(format!("{upcase}_PRIMARY_PORT"), port.to_string());
        }

        let metrics = self
            .ports
            .as_ref()
            .and_then(|p| p.metrics.as_ref())
            .map(|m| m.port());
        if let Some(port) = metrics {
            vars.insert("METRICS_PORT".to_string(), port.to_string());
            vars.insert(format!("{upcase}_METRICS_PORT"), port.to_string());
        }

        if let Some(port) = self.ports.as_ref().and_then(|p| p.config) {
            vars.insert("CONFIG_PORT".to_string(), port.to_string());
            vars.insert(format!("{upcase}_CONFIG_PORT"), port.to_string());
        }

        vars
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

        // If .routing.paths, then also .routing.domain must be set
        for route in self.routing.iter() {
            if !route.paths.is_empty() && route.domain.is_none() {
                return Err(anyhow::Error::msg(
                    "You must specify .paths alongside .domain within .routing, or else we don't \
                     know which domain to modify!",
                ));
            }
        }

        Ok(())
    }
}
