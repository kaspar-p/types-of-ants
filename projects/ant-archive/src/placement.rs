use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use ant_archive_storage_client::AntArchiveStorageNodeClient;
use hashring::HashRing;
use tracing::{debug, error, info, warn};

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
        if let Some((node_id, protocol)) = state
            .db
            .get_storage_node_by_node_name_or_id(&ep.node)
            .await?
        {
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
}

// #[derive(Debug, Clone)]
// pub(crate) enum PlacementRole {
//     Replication,
//     ErrorCorrection(ErrorCorrectionRole),
// }

// #[derive(Debug, Clone)]
// pub(crate) enum ErrorCorrectionRole {
//     Data,
//     Parity,
// }

/// Return a vector of nodes to place onto, `num_placements_to_find` in length.
#[tracing::instrument(skip(state))]
async fn calculate_placements(
    state: &AntArchiveState,

    group_id: &str,
    size: usize,

    num_placements_to_find: i32,

    required_node: Option<&str>,
    disqualified: &HashSet<String>,
) -> Result<Vec<Placement>, AntArchiveError> {
    let storage_nodes = resolve_storage_nodes(&state).await?;
    let storage_nodes_len = storage_nodes.len();

    let mut placements = vec![];

    let mut available_nodes: HashRing<HashRingNode> = HashRing::new();

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
                info!("Forcing the use of {} ({})", node.node_id, node.host_id);
                placements.push(Placement { node: node.client });
                continue;
            }
        }

        if bytes_stored + size as i64 <= capacity_bytes {
            debug!(
                "qualified: {} because {bytes_stored} ({}) + {size} ({}) is less than {capacity_bytes} ({})",
                node.to_string(),
                humansize::format_size_i(bytes_stored, humansize::DECIMAL),
                humansize::format_size_i(size, humansize::DECIMAL),
                humansize::format_size_i(capacity_bytes, humansize::DECIMAL)
            );
            available_nodes.add(node);
        } else {
            debug!(
                "disqualified: {} because {bytes_stored} ({}) + {size} ({}) is more than {capacity_bytes} ({})",
                node.to_string(),
                humansize::format_size_i(bytes_stored, humansize::DECIMAL),
                humansize::format_size_i(size, humansize::DECIMAL),
                humansize::format_size_i(capacity_bytes, humansize::DECIMAL)
            );
        }
    }

    if num_placements_to_find - placements.len() as i32 > available_nodes.len() as i32 {
        error!(
            "Asked for {num_placements_to_find} ({} forced), but only {}/{} available",
            placements.len(),
            available_nodes.len(),
            storage_nodes_len
        );
        return Err(AntArchiveError::InsufficientStorage);
    }

    let mut ring_iter = match available_nodes.get_with_replicas(&group_id, NUM_REPLICATION - 1) {
        Some(ring) => ring,
        None => return Err(AntArchiveError::InsufficientStorage),
    }
    .into_iter();

    while (placements.len() as i32) < num_placements_to_find {
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
                })
            }

            None => {
                return Err(AntArchiveError::InternalServerError(
                    "ANT-ERR-134",
                    Some(anyhow::Error::msg(format!(
                        "failed to place object on placement ring (l={}) after \
                        ensuring enough space (l={}) was available.",
                        available_nodes.len(),
                        placements.len()
                    ))),
                ));
            }
        }
    }

    if placements.is_empty() {
        return Err(AntArchiveError::InsufficientStorage);
    }

    Ok(placements)
}

pub(crate) const NUM_REPLICATION: usize = 3;

/// Here, id means the object or chunk or whatever bytes need to be placed.
/// It's compared to the hashed identities of the nodes to determine ring placement.
pub(crate) async fn place_group(
    state: &AntArchiveState,

    id: &str,
    size: usize,

    num_placements_to_find: i32,

    required_node: Option<&str>,
) -> Result<Vec<Placement>, AntArchiveError> {
    return calculate_placements(
        state,
        id,
        size,
        num_placements_to_find,
        required_node,
        &HashSet::new(),
    )
    .await;
}

/// If replication fails to a node, we want to find a replacement that excludes that node
pub(crate) async fn find_replacement(
    state: &AntArchiveState,

    id: &str,
    size: usize,

    disqualified: &HashSet<String>,
) -> Result<Placement, AntArchiveError> {
    info!("Finding replacement!");
    let placement = calculate_placements(state, id, size, 1, None, disqualified)
        .await?
        .remove(0);

    return Ok(placement);
}
