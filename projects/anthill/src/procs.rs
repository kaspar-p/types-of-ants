use std::{collections::HashMap, path::PathBuf};

use ant_library::sd::writer::ServiceDiscoveryWriter;
use anthill_manifest::AnthillManifest;
use anyhow::Context;

/// Spawn `.anthill/dev.sh`, handle Consul registration/deregistration, and wait
/// for the process to exit or Ctrl+C.
pub async fn spawn_and_wait(
    project_dir: &PathBuf,
    manifest: &AnthillManifest,
    env: &HashMap<String, String>,
    args: &[String],
) -> Result<(), anyhow::Error> {
    let dev_sh = project_dir.join(".anthill").join("dev.sh");
    anyhow::ensure!(
        dev_sh.exists(),
        "{} has no .anthill/dev.sh",
        manifest.project
    );

    let consul = maybe_register(manifest, env).await;

    let mut child = tokio::process::Command::new("bash")
        .arg(&dev_sh)
        .args(args)
        .envs(env)
        .current_dir(project_dir)
        .spawn()
        .context("failed to spawn dev.sh")?;

    tokio::select! {
        _ = child.wait() => {}
        _ = tokio::signal::ctrl_c() => {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    if let Some(sd) = consul {
        if let Err(e) = sd.deregister_local_service(&manifest.project).await {
            tracing::warn!("ANT-ERR-090: Failed to deregister {} from Consul: {e}", manifest.project);
        }
    }

    Ok(())
}

/// Registers the service with Consul if both a matchmaker port and primary port
/// are configured. Returns the writer for deregistration on exit.
///
/// Fails gracefully: prints a warning and returns None rather than aborting.
pub async fn maybe_register(
    manifest: &AnthillManifest,
    env: &HashMap<String, String>,
) -> Option<ServiceDiscoveryWriter> {
    if manifest.project == "ant-matchmaker" {
        eprintln!("skipping Consul registration for ant-matchmaker (that's the Consul instance)");
        return None;
    }

    let matchmaker_port: u16 = env
        .get("ANT_MATCHMAKER_HTTP_PORT")
        .and_then(|p| p.parse().ok())?;

    let primary_port: u16 = manifest.ports.as_ref()?.primary?;

    let sd = ServiceDiscoveryWriter::new(matchmaker_port);

    if !sd.healthy().await {
        eprintln!(
            "warning: could not reach Consul on port {matchmaker_port} — \
             {} will start but won't be discoverable.\n\
             Run `ah dev ant-matchmaker` in another terminal first.",
            manifest.project
        );
        return None;
    }

    if let Err(e) = sd
        .register_local_service(&manifest.project, primary_port)
        .await
    {
        eprintln!(
            "warning: failed to register {} with Consul: {e}",
            manifest.project
        );
        return None;
    }

    tracing::debug!(
        "registered {}:{} with local Consul",
        manifest.project,
        primary_port
    );

    Some(sd)
}
