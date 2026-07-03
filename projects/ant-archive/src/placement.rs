use std::collections::HashMap;

use rand::{rngs::OsRng, seq::SliceRandom};
use tracing::{debug, info, warn};

use crate::{storage_client::AntArchiveStorageNodeClient, AntArchiveError, AntArchiveState};

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

pub async fn resolve_storage_nodes(
    state: &AntArchiveState,
) -> Result<Vec<AntArchiveStorageNodeClient>, AntArchiveError> {
    let creds = get_client_credentials()?;
    let endpoints = state.sd.resolve_all("ant-archive-storage").await;

    let mut clients = Vec::new();
    for ep in &endpoints {
        let (username, password) = creds.get(&ep.node).ok_or(anyhow::Error::msg(format!(
            "No credentials for node: {}",
            ep.node
        )))?;
        if let Some((node_id, protocol)) = state.db.get_storage_node_by_node_name(&ep.node).await? {
            clients.push(AntArchiveStorageNodeClient::new(
                node_id,
                ep.node.clone(),
                format!("{protocol}://{}:{}", ep.address, ep.port),
                username,
                password,
            ));
        } else {
            warn!(
                "Failed to associate [{}] with any host_id of a storage node!",
                ep.node
            )
        }
    }

    Ok(clients)
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
    num_placements_to_find: usize,

    new_object_size_bytes: i64,
    required_node: Option<&str>,
    disqualified: &[String],
) -> Result<Vec<Placement>, AntArchiveError> {
    let storage_nodes = resolve_storage_nodes(&state).await?;

    let mut placements = vec![];

    let mut available_nodes = vec![];
    for node in storage_nodes {
        let (_, capacity_bytes) = state
            .db
            .describe_storage_node(&node.node_id)
            .await?
            .expect("storage node not found");
        let bytes_stored = state.db.bytes_stored_on_node(&node.node_id).await?;

        if let Some(req) = &required_node {
            if *req == node.host_id || *req == node.node_id {
                info!("Forcing the use of {} {}", node.host_id, node.node_id);
                placements.push(Placement {
                    node: node,
                    role: PlacementRole::Replication,
                });
                continue;
            }
        }

        if bytes_stored + new_object_size_bytes <= capacity_bytes {
            available_nodes.push(node);
        }
    }

    // Choose 3, but placements might have required_node in it already.
    let to_choose = num_placements_to_find - placements.len();
    if to_choose > 0 {
        info!("Choosing {to_choose} placements randomly...");
        for node in available_nodes.choose_multiple(&mut OsRng, to_choose) {
            placements.push(Placement {
                node: node.clone(),
                role: PlacementRole::Replication,
            });
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
    new_object_size_bytes: i64,
    required_node: Option<&str>,
) -> Result<Vec<Placement>, AntArchiveError> {
    return calculate_placement(
        state,
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
    new_object_size_bytes: i64,
    disqualified: &[String],
) -> Result<Placement, AntArchiveError> {
    info!("Finding replacement!");
    let placement = calculate_placement(state, 1, new_object_size_bytes, None, disqualified)
        .await?
        .remove(0);
    return Ok(placement);
}
