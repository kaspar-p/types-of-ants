use std::{collections::HashMap, hash::Hash};

use ant_archive_storage_client::AntArchiveStorageNodeClient;
use hashring::HashRing;
use tracing::{info, warn};

use crate::{AntArchiveError, AntArchiveState};

/// Expects a file that's newline delimited lines that look like:
///     {hostname}:{username}:{password}
/// where each are templated, for example:
///     myhost:user1:pass1
///
/// Returns a hashmap mapping from hostname to (username, password)
fn get_client_credentials() -> Result<HashMap<String, (String, String)>, anyhow::Error> {
    let content = ant_library::secret::load_secret("ant_archive_storage_client_auths")?;

    let mut map = HashMap::new();
    for (i, line) in content.split("\n").enumerate() {
        let mut line_content = line.split(":");

        let hostname = line_content
            .next()
            .ok_or(anyhow::Error::msg(format!("Line {i} had no hostname")))?;
        let username = line_content
            .next()
            .ok_or(anyhow::Error::msg(format!("Line {i} had no username")))?;
        let password = line_content
            .next()
            .ok_or(anyhow::Error::msg(format!("Line {i} had no password")))?;

        map.insert(
            hostname.to_string(),
            (username.to_string(), password.to_string()),
        );
    }

    Ok(map)
}

#[derive(Debug, Clone)]
pub struct HashRingNode {
    pub node_id: String,
    pub host_id: String,
    pub client: AntArchiveStorageNodeClient,
}

impl ToString for HashRingNode {
    fn to_string(&self) -> String {
        format!("{} ({})", self.host_id, self.node_id)
    }
}

impl Hash for HashRingNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node_id.hash(state);
    }
}

pub async fn resolve_storage_nodes(
    state: &AntArchiveState,
) -> Result<HashRing<HashRingNode>, AntArchiveError> {
    let creds = get_client_credentials()?;
    let endpoints = state.sd.resolve_all("ant-archive-storage").await;

    // Hash the IDs of each storage node into the ring.
    let mut ring = HashRing::<HashRingNode>::new();

    for ep in &endpoints {
        let (username, password) = creds.get(&ep.node).ok_or(anyhow::Error::msg(format!(
            "No credentials for node: {}",
            ep.node
        )))?;
        if let Some((node_id, protocol)) = state.db.get_storage_node_by_node_name(&ep.node).await? {
            ring.add(HashRingNode {
                node_id: node_id.clone(),
                host_id: ep.node.clone(),
                client: AntArchiveStorageNodeClient::new(
                    node_id.clone(),
                    ep.node.clone(),
                    format!("{protocol}://{}:{}", ep.address, ep.port),
                    username,
                    password,
                ),
            });
        } else {
            warn!(
                "Failed to associate [{}] with any host_id of a storage node!",
                ep.node
            )
        }
    }

    Ok(ring)
}

#[derive(Clone)]
pub(crate) struct Placement {
    pub node: AntArchiveStorageNodeClient,
    pub role: PlacementRole,
}

#[derive(Debug, Clone)]
pub(crate) enum PlacementRole {
    Replication,
    ErrorCorrection(ErrorCorrectionRole),
}

#[derive(Debug, Clone)]
pub(crate) enum ErrorCorrectionRole {
    Data,
    Parity,
}

#[tracing::instrument(skip(state))]
async fn calculate_placement(
    state: &AntArchiveState,

    object_key: &str,
    num_placements_to_find: usize,

    new_object_size_bytes: i64,
    required_node: Option<&str>,
    disqualified: &[String],
) -> Result<Vec<Placement>, AntArchiveError> {
    let storage_nodes = resolve_storage_nodes(&state).await?;

    let mut placements = vec![];

    let mut available_nodes = HashRing::new();

    for node in storage_nodes {
        let (_, capacity_bytes) = state
            .db
            .describe_storage_node(&node.node_id)
            .await?
            .expect("storage node not found");
        let bytes_stored = state.db.bytes_stored_on_node(&node.node_id).await?;

        if disqualified.contains(&node.node_id) {
            continue;
        }

        if let Some(req) = &required_node {
            if *req == node.host_id || *req == node.node_id {
                info!("Forcing the use of {} {}", node.host_id, node.node_id);
                placements.push(Placement {
                    node: node.client,
                    role: PlacementRole::Replication,
                });
                continue;
            }
        }

        if bytes_stored + new_object_size_bytes <= capacity_bytes {
            available_nodes.add(node);
        }
    }

    let mut ring_iter = match available_nodes.get_with_replicas(&object_key, NUM_REPLICATION - 1) {
        Some(ring) => ring,
        None => return Err(AntArchiveError::InsufficientStorage),
    }
    .into_iter();

    while placements.len() < num_placements_to_find {
        info!("Choosing [idx={}] placement...", placements.len());

        let ring_node = ring_iter.next();
        match ring_node {
            Some(ring_node) => {
                // Skip placements we've already made!
                if placements
                    .iter()
                    .any(|p| p.node.node_id == ring_node.node_id)
                {
                    continue;
                }

                placements.push(Placement {
                    node: ring_node.client.clone(),
                    role: PlacementRole::Replication,
                })
            }
            None => {
                warn!("Ran out of ring nodes at idx={}", placements.len());
                break;
            }
        }
    }

    if placements.is_empty() {
        return Err(AntArchiveError::InsufficientStorage);
    }

    Ok(placements)
}

pub(crate) const NUM_REPLICATION: usize = 3;

#[tracing::instrument(skip(state))]
pub(crate) async fn place_new_object(
    state: &AntArchiveState,
    object_key: &str,
    new_object_size_bytes: i64,
    required_node: Option<&str>,
) -> Result<Vec<Placement>, AntArchiveError> {
    return calculate_placement(
        state,
        object_key,
        NUM_REPLICATION,
        new_object_size_bytes,
        required_node,
        &[],
    )
    .await;
}

/// If replication fails to a node, we want to find a replacement that excludes that node
#[tracing::instrument(skip(state))]
pub(crate) async fn find_replacement(
    state: &AntArchiveState,
    object_key: &str,
    new_object_size_bytes: i64,
    disqualified: &[String],
) -> Result<Placement, AntArchiveError> {
    info!("Finding replacement!");
    let placement = calculate_placement(
        state,
        object_key,
        1,
        new_object_size_bytes,
        None,
        disqualified,
    )
    .await?
    .remove(0);

    return Ok(placement);
}
