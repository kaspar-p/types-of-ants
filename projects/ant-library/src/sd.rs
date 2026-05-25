use rs_consul::{Consul, ConsulError, GetServiceNodesRequest, QueryOptions};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use crate::service::Service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub address: String,
    pub port: u16,
}

/// Use typesofants' Consul deployments to discover the (IP, Port) pairs of services on other hosts.
pub struct ServiceDiscovery {
    consul: Arc<Consul>,

    /// Map of service-id to the task that keeps cache refreshed
    refreshers: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,

    /// Map of service-id and service endpoints
    cache: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
}

impl ServiceDiscovery {
    pub fn new(ant_matchmaker_http_port: u16) -> Self {
        info!("init service discovery on port {ant_matchmaker_http_port}");
        let consul = Consul::new(rs_consul::Config {
            address: format!("http://localhost:{ant_matchmaker_http_port}"),
            ..Default::default()
        });

        Self {
            consul: Arc::new(consul),
            refreshers: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn resolve(&self, service: &Service) -> Option<ServiceEndpoint> {
        let service_name = &service.to_string();

        // Fast path: cache hit
        {
            info!("x1!");
            let cache = self.cache.read().await;
            if let Some(endpoints) = cache.get(service_name.as_str()) {
                info!("x2!");
                info!("resolved [{}] hit cache", service);
                return endpoints.first().cloned();
            }
        }
        info!("x3!");

        // Slow path: first resolution — fetch, cache, start watcher
        info!("first resolve: [{}], starting background task...", service);
        self.ensure_refreshing(&service).await;

        info!("x4!");

        let cache = self.cache.read().await;
        cache.get(service_name.as_str())?.first().cloned()
    }

    async fn fetch_endpoints(
        consul: Arc<Consul>,
        cache: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
        service: &Service,
        index: Option<u64>,
    ) -> Result<u64, ConsulError> {
        info!("Fetching remote endpoints of [{}]", service);
        let service_name = service.to_string();

        let nodes = consul
            .get_service_nodes(
                GetServiceNodesRequest {
                    service: &service_name,
                    passing: true,
                    ..Default::default()
                },
                Some(QueryOptions {
                    // If index is requested, then wait 5 seconds as a polling query.
                    index: index,
                    wait: index.map(|_| Duration::from_secs(5)),
                    ..Default::default()
                }),
            )
            .await?;
        debug!("consul fetch nodes: {:?}", nodes);
        assert!(nodes.index > 0, "Hashicorp documentation insists that a returned index will always be greater than zero.");

        let endpoints: Vec<ServiceEndpoint> = nodes
            .response
            .into_iter()
            .map(|node| ServiceEndpoint {
                address: node.service.address,
                port: node.service.port,
            })
            .collect();

        let endpoints_str = endpoints
            .iter()
            .map(|e| format!("{}:{}", e.address, e.port))
            .collect::<Vec<_>>()
            .join(", ");
        info!("Discovered [{service}] => [{endpoints_str}]");
        cache.write().await.insert(service_name, endpoints);

        // The documentation: https://developer.hashicorp.com/consul/api-docs/features/blocking#implementation-details
        // lists this as a failure mode: if the index returned is every less than the previous, reset the entire counter.
        if let Some(index) = index {
            if nodes.index < index {
                return Ok(0);
            }
        }

        Ok(nodes.index)
    }

    /// Spawns a background task to watch updates for `service_id`, or does nothing if already exists.
    async fn ensure_refreshing(&self, service: &Service) -> () {
        let service_name = service.to_string();

        // Initial fetch, ignore errors if they happen
        info!("Fetching initial endpoints...");
        if let Err(e) = ServiceDiscovery::fetch_endpoints(
            self.consul.clone(),
            self.cache.clone(),
            &service,
            None,
        )
        .await
        {
            error!("Failed to fetch {service} endpoints, but ignoring error: {e}")
        }

        info!("Spawning background worker...");
        // Spawn background watcher (blocking query loop)
        let consul = self.consul.clone();
        let cache = self.cache.clone();
        let service2 = service.clone();
        let handle = tokio::spawn(async move {
            let service3 = service2;
            let mut index = 0u64;
            loop {
                match ServiceDiscovery::fetch_endpoints(
                    consul.clone(),
                    cache.clone(),
                    &service3,
                    Some(index),
                )
                .await
                {
                    Ok(new_index) => index = new_index,
                    Err(e) => {
                        error!("Failed to fetch {} endpoints: {e}", service2);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        self.refreshers.write().await.insert(service_name, handle);
    }

    pub async fn stop_refreshing(&self, service: &Service) {
        let service_name = service.to_string();

        let mut refreshers = self.refreshers.write().await;
        if let Some(handle) = refreshers.remove(&service_name) {
            handle.abort();
        }
        self.cache.write().await.remove(&service_name);
    }

    pub async fn shutdown(&self) {
        let mut refreshers = self.refreshers.write().await;
        for (_, handle) in refreshers.drain() {
            handle.abort();
        }
    }
}
