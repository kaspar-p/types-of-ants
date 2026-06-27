use std::{
    collections::{HashMap, HashSet},
    fs::File,
    hash::Hash,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::host_architecture::HostArchitecture;

#[derive(Debug, Serialize, Deserialize)]
pub struct Services {
    pub hosts: HashMap<String, HostConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostConfig {
    /// Sometimes I step on a host...
    pub inactive: Option<bool>,
    /// Services owned and run on my own servers have ant-host-agent running there, and can auto-deploy.
    /// Others are run on hardware I don't control. These are SKIPPED from being part of any host group for deployments.
    pub auto_deployable: bool,
    /// The architecture of the host, for building binaries and such.
    pub architecture: HostArchitecture,
    /// The service instances running on that host.
    pub services: Vec<ServiceInstance>,
}

impl HostConfig {
    pub fn ineligible_for_deployments(&self) -> bool {
        self.inactive.unwrap_or(false) || !self.auto_deployable
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceInstance {
    /// The set of secrets/configuration this gets.
    pub env: ServiceEnv,

    /// The ID of the project running, e.g. "ant-data-farm"
    pub project: String,

    /// Override the project ID as the string that keeps the running instance of the project unique on a host.
    ///
    /// For example, the project might be "ant-collecting-the-database" for metrics collection but
    /// there may be multiple databases ("ant-data-farm", "ant-backing-it-up", ...) on the same host,
    /// and each needs its own metrics collection. The `service_id` might therefore include the
    /// _target service_ for the metrics collector, "ant-data-farm", being then:
    /// ```ignore
    /// ant-collecting-the-database.ant-data-farm
    /// ```
    /// or something.
    ///
    /// The directory on-host will look like ~/service/<service_id>/<version>/...
    /// For example:
    ///
    /// ```ignore
    /// ~/service/
    ///     ant-collecting-the-database.ant-data-farm/
    ///         ...
    ///     ant-collecting-the-database.ant-backing-it-up/
    ///         ...
    /// ```
    ///
    /// DO NOT CHANGE THIS FIELD AFTER SETTING IT. It will likely lead to deploy failures, since the
    /// replacement step of the deployment will fail to turn off the previous version of the service,
    /// and it'll lead to port bind issues in the small, and two-deputies in the large.
    pub service_id: Option<String>,

    /// Additional environment variables to apply to this instance of the service.
    /// This is used so we can deploy the same service, with slightly different tweaks, to each host.
    ///
    /// For example, a server that exports postgres-specific metrics would sit alongside each database,
    /// and needs to be configured to collect metrics from that specific database. There may even be
    /// multiple on the same machine, but I have to think about that some more.
    pub additional_vars: Option<HashMap<String, String>>,
}

impl ServiceInstance {
    pub fn id<'a>(&'a self) -> &'a str {
        match &self.service_id {
            None => &self.project,
            Some(instance) => instance,
        }
    }
}

impl Hash for ServiceInstance {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ServiceEnv {
    /// Gets the beta.build.cfg environment variable set, and beta secrets
    #[serde(rename = "beta")]
    Beta,

    /// Gets the prod.build.cfg environment variable set, and prod secrets
    #[serde(rename = "prod")]
    Prod,
}

impl Services {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        for (_, v) in &self.hosts {
            let seen_services = HashSet::<&str>::new();
            for service in &v.services {
                let id = service.id();
                if seen_services.contains(&id) {
                    return Err(anyhow::Error::msg(format!(
                        "Cannot have more than 1 service with ID: {id}"
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn from_path(p: &Path) -> Result<Self, anyhow::Error> {
        let f = File::open(p)?;
        let content: Self = serde_json::from_reader(&f)?;
        content.validate()?;
        Ok(content)
    }

    pub fn list_service_ids<'a>(&'a self) -> Vec<&'a str> {
        let mut service_ids = HashSet::<&str>::new();

        for (_, host) in &self.hosts {
            if host.ineligible_for_deployments() {
                continue;
            }

            for svc in &host.services {
                service_ids.insert(svc.id());
            }
        }

        service_ids.into_iter().collect()
    }

    pub fn list_hosts_with_project<'a>(
        &'a self,
        project: &str,
    ) -> Vec<(&'a str, &'a ServiceInstance)> {
        let mut hosts = HashSet::<(&'a str, &'a ServiceInstance)>::new();

        for (host_id, host) in &self.hosts {
            if host.ineligible_for_deployments() {
                continue;
            }

            for host_service in &host.services {
                if host_service.project == project {
                    hosts.insert((host_id, host_service));
                }
            }
        }

        hosts.into_iter().collect()
    }

    pub fn service_instance<'a>(
        &'a self,
        service_id: &str,
        host_id: &str,
    ) -> Option<&'a ServiceInstance> {
        for (host, host_config) in &self.hosts {
            if host != host_id || host_config.ineligible_for_deployments() {
                continue;
            }

            for host_service in &host_config.services {
                if host_service.id() == service_id {
                    return Some(&host_service);
                }
            }
        }

        return None;
    }

    pub fn list_architectures_for_service<'a>(
        &'a self,
        service_id: &str,
    ) -> HashSet<HostArchitecture> {
        let arches: HashSet<HostArchitecture> = self
            .list_hosts_with_service(service_id)
            .into_iter()
            .map(|(h, _)| {
                self.hosts
                    .get(h)
                    .expect(&format!("can't find host bloc for host {service_id}"))
                    .architecture
                    .clone()
            })
            .collect();

        arches.into_iter().collect()
    }

    pub fn list_hosts_with_service<'a>(
        &'a self,
        service_id: &str,
    ) -> Vec<(&'a str, &'a ServiceInstance)> {
        let mut hosts = HashSet::<(&'a str, &'a ServiceInstance)>::new();

        for (host_id, host) in &self.hosts {
            if host.ineligible_for_deployments() {
                continue;
            }

            for host_service in &host.services {
                if host_service.id() == service_id {
                    hosts.insert((host_id, host_service));
                }
            }
        }

        hosts.into_iter().collect()
    }
}
