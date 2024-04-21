use ant_host_agent::clients::{Host, HostAgentClient};
use reqwest::StatusCode;

use super::ping::StatusData;

const HOSTS: &[&str] = &[
    "antworker000.hosts.typesofants.org",
    "antworker001.hosts.typesofants.org",
    "antworker002.hosts.typesofants.org",
];

/**
 * Ping the relevant URLs to see if they are up
 *
 * TODO: make the request non-blocking
 */
pub async fn ping_host() -> Vec<StatusData> {
    let mut metrics: Vec<StatusData> = Vec::new();
    for host in HOSTS {
        let agent = HostAgentClient::connect(Host::new(host.to_string(), 7000)).unwrap();

        let start_timestamp = std::time::SystemTime::now().into();
        let res = agent.ping().await;
        let metric = match res {
            Err(_) => StatusData::new(
                agent.host.http_endpoint(),
                start_timestamp,
                false,
                StatusCode::from_u16(500).unwrap(),
            ),
            Ok(_) => StatusData::new(
                agent.host.http_endpoint(),
                start_timestamp,
                true,
                StatusCode::from_u16(200).unwrap(),
            ),
        };

        metrics.push(metric);
    }

    metrics
}
