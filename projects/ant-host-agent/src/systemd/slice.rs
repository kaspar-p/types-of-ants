use anyhow::Context;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, instrument, warn};
use zbus_systemd::zbus;

use crate::{
    state::AntHostAgentState,
    systemd::{restart_unit, SLICE_NAME},
};

fn slice_content() -> &'static str {
    "[Unit]
Description=All typesofants services

[Slice]

[Install]
WantedBy=multi-user.target
"
}

/// On startup, ensure the typesofants.slice systemd slice exists, for all other projects to use.
#[instrument(skip(state))]
pub async fn ensure_slice(state: AntHostAgentState) -> Result<(), anyhow::Error> {
    let conn = zbus::Connection::system()
        .await
        .context("systemd connection")?;
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .context("manager init")?;

    // Attempt to find slice
    let units = manager
        .list_units_by_patterns(vec!["active".to_string()], vec![SLICE_NAME.to_string()])
        .await
        .context("list slices")?;
    if units.iter().find(|u| u.0 == SLICE_NAME).is_some() {
        info!("Found existing slice {SLICE_NAME}");
        return Ok(());
    }

    // Create slice file

    let slices_dir = state.archive_root_dir.join("slices");
    tokio::fs::create_dir_all(&slices_dir)
        .await
        .context("creating slices dir")?;

    let slices_path = slices_dir.join(SLICE_NAME);

    let mut slices_file = tokio::fs::File::create(&slices_path)
        .await
        .context("creating slices file")?;

    slices_file
        .write_all(slice_content().as_bytes())
        .await
        .context("write slice content")?;

    // Enable new slice file

    let enable = manager
        .enable_unit_files(vec![slices_path.to_str().unwrap().to_string()], false, true)
        .await;
    match enable {
        Ok(unit) => {
            info!("Enabled slice: {:?}", unit);
        }
        Err(zbus::Error::MethodError(name, _, _))
            if name == "org.freedesktop.systemd1.NoSuchUnit" =>
        {
            warn!("ANT-ERR-037: No such unit file: {}", slices_path.display());
        }
        Err(e) => {
            error!(
                "ANT-ERR-038: Failed to enable unit file: {}, {}",
                slices_path.display(),
                e
            );
        }
    }

    restart_unit(&manager, SLICE_NAME).await??;

    info!("Ensured {SLICE_NAME} loaded.");

    Ok(())
}
