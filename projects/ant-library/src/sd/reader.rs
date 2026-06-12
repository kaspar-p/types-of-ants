use rs_consul::{Consul, ConsulError, GetServiceNodesRequest, QueryOptions};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub node: String,
    pub address: String,
    pub port: u16,
}

impl ToString for ServiceEndpoint {
    fn to_string(&self) -> String {
        format!("{} ({}:{})", self.node, self.address, self.port)
    }
}

#[derive(Clone)]
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
        // rs-consul builds a hyper-rustls HTTPS connector even for HTTP addresses.
        // rustls 0.23 requires an explicit CryptoProvider; install ring here so any
        // caller doesn't need to know about this detail.
        let _ = rustls::crypto::ring::default_provider().install_default();
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

    pub async fn resolve(&self, service: &str) -> Option<ServiceEndpoint> {
        let service_name = &service.to_string();

        // Fast path: cache hit
        {
            let cache = self.cache.read().await;
            if let Some(endpoints) = cache.get(service_name.as_str()) {
                return endpoints.first().cloned();
            }
        }

        // Slow path: first resolution — fetch, cache, start watcher
        info!("first resolve: [{}], starting background task...", service);
        self.ensure_refreshing(&service).await;

        let cache = self.cache.read().await;
        cache.get(service_name.as_str())?.first().cloned()
    }

    async fn fetch_endpoints(
        consul: Arc<Consul>,
        cache: Arc<RwLock<HashMap<String, Vec<ServiceEndpoint>>>>,
        service: &str,
        index: Option<u64>,
    ) -> Result<u64, ConsulError> {
        info!("Fetching remote endpoints of [{}]", service);

        let nodes = consul
            .get_service_nodes(
                GetServiceNodesRequest {
                    service: service,
                    passing: true,
                    ..Default::default()
                },
                Some(QueryOptions {
                    // If index is requested, then wait for a polling query
                    index: index,
                    wait: index.map(|_| Duration::from_secs(30)),
                    ..Default::default()
                }),
            )
            .await?;
        if nodes.index <= 0 {
            error!("Hashicorp documentation insists that a returned index will always be greater than zero, got: {}", nodes.index);
        }

        let endpoints: Vec<ServiceEndpoint> = nodes
            .response
            .into_iter()
            .map(|node| {
                // The "service address" is really an override, only if the client sets it
                let address = Some(node.service.address)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(node.node.address);

                ServiceEndpoint {
                    node: node.node.node,
                    address,
                    port: node.service.port,
                }
            })
            .collect();

        let endpoints_str = endpoints
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        info!("Discovered [{service}] => [{endpoints_str}]");
        cache.write().await.insert(service.to_string(), endpoints);

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
    async fn ensure_refreshing(&self, service: &str) -> () {
        {
            // Idempotency.
            if self.refreshers.read().await.contains_key(service) {
                return;
            }
        }

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
            error!("Failed to fetch [{service}] endpoints, but ignoring error: {e}")
        }

        info!("Spawning background worker...");
        // Spawn background watcher (blocking query loop)
        let consul = self.consul.clone();
        let cache = self.cache.clone();
        let service2 = service.to_string().clone();
        let handle = tokio::spawn(async move {
            let service3 = service2.clone();
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
                    Err(ConsulError::TimeoutExceeded(_)) => {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        error!("Failed to fetch [{}] endpoints: {e}", service2);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        self.refreshers
            .write()
            .await
            .insert(service.to_string(), handle);
    }

    pub async fn resolve_all(&self, service: &str) -> Vec<ServiceEndpoint> {
        let service_name = service.to_string();

        {
            let cache = self.cache.read().await;
            if let Some(endpoints) = cache.get(&service_name) {
                return endpoints.clone();
            }
        }

        self.ensure_refreshing(service).await;

        let cache = self.cache.read().await;
        cache.get(&service_name).cloned().unwrap_or_default()
    }

    pub async fn stop_refreshing(&self, service: &str) {
        let mut refreshers = self.refreshers.write().await;
        if let Some(handle) = refreshers.remove(service) {
            handle.abort();
        }
        self.cache.write().await.remove(service);
    }

    pub async fn shutdown(&self) {
        let mut refreshers = self.refreshers.write().await;
        for (_, handle) in refreshers.drain() {
            handle.abort();
        }
    }
}
