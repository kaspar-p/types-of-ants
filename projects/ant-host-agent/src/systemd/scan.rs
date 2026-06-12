use std::path::Path;

use anthill_manifest::AnthillManifest;
use anyhow::Context;
use tracing::{debug, info, instrument, warn};
use zbus_systemd::{
    systemd1::{ServiceProxy, UnitProxy},
    zbus,
};

use crate::{
    state::AntHostAgentState,
    systemd::SLICE_NAME,
};

/// On startup, scan for all ACTIVE systemd units in the typesofants slice and register them with
/// Consul. Failures are non-fatal: the service may not be healthy yet or Consul may not be up.
#[instrument(skip(state))]
pub async fn find_active_services(state: AntHostAgentState) -> Result<(), anyhow::Error> {
    let conn = zbus::Connection::system()
        .await
        .context("systemd connection")?;
    let manager = zbus_systemd::systemd1::ManagerProxy::new(&conn)
        .await
        .context("manager init")?;

    let units = manager
        .list_units_by_patterns(vec!["active".to_string()], vec!["*.service".to_string()])
        .await
        .context("list units")?;

    let mut registered = Vec::new();

    info!("Scanning units to find typesofants related services...");
    for unit in units {
        debug!("Considering: {:?}", unit);
        let (unit_name, _, _, _, _, _, unit_path, ..) = unit;
        let unit_proxy = UnitProxy::builder(&conn).path(&unit_path)?.build().await?;

        let fragment_path = match unit_proxy.fragment_path().await {
            Ok(p) => p,
            Err(e) => {
                debug!("Skipping {unit_name} due to no FragmentPath property: {e}");
                continue;
            }
        };

        let service_file = Path::new(&fragment_path);

        let service_file_symlink = match service_file.canonicalize() {
            Ok(d) => {
                debug!(
                    "{} was symlinked to: {}",
                    service_file.display(),
                    d.display()
                );
                d
            }
            Err(e) => {
                debug!("Skipping {unit_name} after following broken symlink: {e}");
                continue;
            }
        };

        let install_dir = match service_file_symlink.parent() {
            Some(d) => d,
            None => {
                debug!(
                    "Skipping {unit_name} due to path {} having no parent.",
                    service_file_symlink.display()
                );
                continue;
            }
        };

        if install_dir
            .ancestors()
            .find(|d| *d == state.install_root_dir)
            .is_none()
        {
            warn!(
                "Service {unit_name} install dir {} does not have install root as ancestor: {}",
                install_dir.display(),
                state.install_root_dir.display()
            );
        }

        let manifest = match AnthillManifest::from_file(&install_dir.join("anthill.json")) {
            Ok(m) => m,
            Err(e) => {
                debug!(
                    "Skipping {unit_name} due to no anthill.json manifest found in {}: {e}",
                    install_dir.display()
                );
                continue;
            }
        };

        let service_proxy = match ServiceProxy::builder(&conn).path(&unit_path)?.build().await {
            Ok(proxy) => proxy,
            Err(e) => {
                debug!("Skipping {unit_name} due to failing to build service proxy: {e}");
                continue;
            }
        };

        match service_proxy.slice().await {
            Ok(s) if s == SLICE_NAME => {
                let service_id = unit_name.strip_suffix(".service").unwrap();
                info!("Found typesofants service: {service_id}");
                let port = manifest.ports.as_ref().and_then(|p| p.primary).unwrap_or(0);
                if let Err(e) = state.sd.register_local_service(service_id, port).await {
                    warn!("Failed to register {service_id} with Consul on startup: {e}");
                }
                registered.push(service_id.to_string());
            }
            r => {
                debug!("Skipping {unit_name} due to not included in {SLICE_NAME}: {r:?}");
                continue;
            }
        }
    }

    info!(
        "Registered [{}] active typesofants services with Consul: {}",
        registered.len(),
        registered.join(", ")
    );

    Ok(())
}
