use ant_library::sd::writer::ServiceDiscoveryWriter;
use anthill_manifest::AnthillManifest;
use anyhow::Context;
use clap_complete::engine::ArgValueCompleter;

use crate::build::GitState;
use crate::complete::complete_projects;

#[derive(clap::Args)]
pub struct DevCmd {
    #[arg(add = ArgValueCompleter::new(complete_projects))]
    project: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

pub async fn dev(cmd: DevCmd) -> Result<(), anyhow::Error> {
    let repo_root = GitState::new().context("failed to find git root")?.root;

    let project_dir = repo_root.join("projects").join(&cmd.project);
    let manifest = AnthillManifest::from_file(&project_dir.join("anthill.json"))
        .context(format!("no anthill.json for project {}", cmd.project))?;

    let dev_sh = project_dir.join(".anthill").join("dev.sh");
    anyhow::ensure!(dev_sh.exists(), "{} has no .anthill/dev.sh", cmd.project);

    let build_cfg = repo_root.join("secrets").join("dev").join("build.cfg");
    let mut env = ant_library::env::env_vars_to_map(&build_cfg)
        .context("failed to read secrets/dev/build.cfg")?;

    env.extend(manifest.to_port_env_vars(&cmd.project));

    env.insert(
        "TYPESOFANTS_SECRET_DIR".to_string(),
        repo_root
            .join("secrets")
            .join("dev")
            .to_string_lossy()
            .into_owned(),
    );
    env.insert(
        "PERSIST_DIR".to_string(),
        project_dir.join("dev-fs").to_string_lossy().into_owned(),
    );

    let consul = maybe_register(&manifest, &env).await;

    let mut child = tokio::process::Command::new("bash")
        .arg(&dev_sh)
        .args(&cmd.args)
        .envs(&env)
        .current_dir(&project_dir)
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
            tracing::warn!("Failed to deregister {} from Consul: {e}", manifest.project);
        }
    }

    Ok(())
}

/// Registers the service with the local Consul instance if both a matchmaker port and primary
/// port are configured. Returns the writer so the caller can deregister on exit.
///
/// Fails gracefully: if Consul is not reachable, prints a clear message and returns None so
/// the service still starts unregistered.
async fn maybe_register(
    manifest: &AnthillManifest,
    env: &std::collections::HashMap<String, String>,
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
            "warning: could not reach local ant-matchmaker on port {matchmaker_port} — {} will \
             start but won't be discoverable by other local services.\nRun `ah dev \
             ant-matchmaker` in another terminal first.",
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
