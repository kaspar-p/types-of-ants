use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::host_architecture::HostArchitecture;

#[derive(Debug, Serialize, Deserialize)]
pub struct Services {
    pub hosts: HashMap<String, HostConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostConfig {
    pub inactive: Option<bool>,
    pub auto_deployable: bool,
    pub architecture: HostArchitecture,
    pub services: Vec<HostService>,
}

impl HostConfig {
    pub fn ineligible(&self) -> bool {
        self.inactive.unwrap_or(false) || !self.auto_deployable
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HostService {
    pub env: ServiceEnv,
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ServiceEnv {
    #[serde(rename = "beta")]
    Beta,
    #[serde(rename = "prod")]
    Prod,
}

impl Services {
    pub fn list_service_ids<'a>(&'a self) -> Vec<&'a str> {
        let mut service_ids = HashSet::<&'a str>::new();

        for (_, host) in &self.hosts {
            if host.ineligible() {
                continue;
            }

            for svc in &host.services {
                service_ids.insert(&svc.service);
            }
        }

        service_ids.into_iter().collect()
    }

    pub fn list_hosts_in_service<'a>(
        &'a self,
        service: &'a str,
    ) -> Vec<(&'a str, &'a HostService)> {
        let mut hosts = HashSet::<(&'a str, &'a HostService)>::new();

        for (host_id, host) in &self.hosts {
            if host.ineligible() {
                continue;
            }

            for host_service in &host.services {
                if host_service.service == service {
                    hosts.insert((host_id, host_service));
                }
            }
        }

        hosts.into_iter().collect()
    }
}
