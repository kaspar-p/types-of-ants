use std::collections::HashMap;

use rand::{rngs::OsRng, seq::SliceRandom};

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
        }
    }

    Ok(clients)
}

pub(crate) struct Placement {
    pub node: AntArchiveStorageNodeClient,
    pub role: PlacementRole,
}

#[derive(Debug)]
pub(crate) enum PlacementRole {
    Replication,
    ErrorCorrection(ErrorCorrectionRole),
}

#[derive(Debug)]
pub(crate) enum ErrorCorrectionRole {
    Data,
    Parity,
}

pub(crate) async fn place_new_object(
    state: &AntArchiveState,
    new_object_size_bytes: i64,
    required_node: Option<&str>,
) -> Result<Vec<Placement>, AntArchiveError> {
    let storage_nodes = resolve_storage_nodes(&state).await?;

    let mut available_nodes = vec![];
    for node in storage_nodes {
        let (_, capacity_bytes) = state
            .db
            .describe_storage_node(&node.node_id)
            .await?
            .expect("storage node not found");
        let bytes_stored = state.db.bytes_stored_on_node(&node.node_id).await?;

        if bytes_stored + new_object_size_bytes <= capacity_bytes {
            available_nodes.push(node);
        }
    }

    let mut placements = vec![];

    if let Some(req) = required_node {
        let pos = available_nodes
            .iter()
            .position(|n| n.node_id == req || n.host_id == req)
            .ok_or_else(|| {
                AntArchiveError::BadRequest(format!(
                    "required storage node '{req}' not found or has no capacity"
                ))
            })?;
        let required = available_nodes.remove(pos);
        placements.push(Placement {
            node: required,
            role: PlacementRole::Replication,
        });
        for node in available_nodes.choose_multiple(&mut OsRng, 2) {
            placements.push(Placement {
                node: node.clone(),
                role: PlacementRole::Replication,
            });
        }
    } else {
        for node in available_nodes.choose_multiple(&mut OsRng, 3) {
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
