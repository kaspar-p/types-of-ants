use std::time::Duration;

use anyhow::Context;
use tokio::time::sleep;
use tracing::{info, instrument};
use zbus_systemd::systemd1::ManagerProxy;

pub mod scan;
pub mod slice;

#[derive(Debug, thiserror::Error)]
pub enum SystemdUnitError {
    #[error("unit failed to start")]
    UnitFailedToStart,
    #[error("unit in unrecognized state [loaded: `{0}`, active: `{1}`] ")]
    UnrecognizedState(String, String),
    #[error("unit took too long to start")]
    UnitTookTooLongToStart,
}

#[instrument(skip(manager))]
pub async fn restart_unit(
    manager: &ManagerProxy<'_>,
    unit_name: &str,
) -> Result<Result<(), SystemdUnitError>, anyhow::Error> {
    info!("Starting unit...");
    manager
        .reload_or_restart_unit(unit_name.to_string(), "replace".to_string())
        .await
        .context("systemd reload")?;

    let mut queued = true;
    while queued {
        info!("Polling for job to start.");
        queued = manager
            .list_jobs()
            .await
            .context("list systemd jobs")?
            .iter()
            .any(|(_, some_unit_name, _, _, _, _)| *some_unit_name == unit_name);

        sleep(Duration::from_millis(500)).await;
    }

    const MAX_TRIES: i32 = 500;
    let mut tries = 0;
    let mut activating = true;
    while activating && tries < MAX_TRIES {
        let units = manager
            .list_units_by_names(vec![unit_name.to_string()])
            .await
            .context("list units")?;
        let unit = units.first().unwrap();
        let (_, _, loaded_state, active_state, _, _, _, _, _, _) = unit;

        info!("Polling for job to activate: {unit:?}");
        activating = match (loaded_state.as_str(), active_state.as_str()) {
            ("loaded", "activating") => true,
            ("loaded", "active") => false,
            (_, "failed") => {
                return Ok(Err(SystemdUnitError::UnitFailedToStart));
            }
            (loaded_state, active_state) => {
                return Ok(Err(SystemdUnitError::UnrecognizedState(
                    loaded_state.to_string(),
                    active_state.to_string(),
                )));
            }
        };

        tries += 1;
        sleep(Duration::from_millis(500)).await;
    }

    if tries >= MAX_TRIES {
        info!("{unit_name} took over {MAX_TRIES} polling attempts to start!");
        return Ok(Err(SystemdUnitError::UnitTookTooLongToStart));
    }

    Ok(Ok(()))
}
