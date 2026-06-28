use tracing::info;

use crate::state::AntZookeeperState;

pub async fn migrate_db(
    state: AntZookeeperState,
    service_id: &str,
    host_id: &str,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("TODO: migrate the db");

    Ok(())
}
