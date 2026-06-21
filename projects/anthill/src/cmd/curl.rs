use ant_library::sd::reader::ServiceDiscovery;
use anyhow::Context;
use clap_complete::engine::ArgValueCompleter;
use tracing::{debug, info};

use crate::complete::complete_projects;
use crate::git::GitState;

#[derive(clap::Args)]
pub struct CurlCmd {
    /// Service to reach, in the form "service", "service:env", or "service:env/path".
    /// Environment defaults to "dev". Examples:
    ///   ant-on-the-web
    ///   ant-on-the-web:prod
    ///   ant-on-the-web:prod/api/ants
    #[arg(add = ArgValueCompleter::new(complete_projects))]
    service_env: String,

    /// Additional arguments forwarded verbatim to curl.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

pub async fn curl(cmd: CurlCmd) -> Result<(), anyhow::Error> {
    let (service, env, path) = parse_arg(&cmd.service_env);

    let (address, port) = resolve(&service, &env).await?;

    let url = format!("http://{}:{}{}", address, port, path);

    debug!("Calling: {url}");

    let mut child = tokio::process::Command::new("curl")
        .arg(&url)
        .arg("--no-progress-meter")
        .arg("--fail-with-body")
        .args(&cmd.args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to curl")?;

    let handles = ant_library::process::prefix_log(&service, &mut child).context("logs")?;

    let status = child.wait().await.context("curl failed")?;
    futures::future::join_all(handles).await;

    info!("{status}");

    std::process::exit(status.code().unwrap_or(1));
}

/// Parses "service", "service:env", or "service:env/path" into (service, env, path).
fn parse_arg(input: &str) -> (String, String, String) {
    let (service_env, path) = match input.find('/') {
        Some(idx) => (&input[..idx], input[idx..].to_string()),
        None => (input, String::new()),
    };

    let (service, env) = match service_env.split_once(':') {
        Some((s, e)) => (s.to_string(), e.to_string()),
        None => (service_env.to_string(), "dev".to_string()),
    };

    (service, env, path)
}

async fn resolve(service: &str, env: &str) -> Result<(String, u16), anyhow::Error> {
    let git = GitState::new()?;
    let build_cfg = git.root.join("secrets").join("dev").join("build.cfg");
    let vars = ant_library::env::env_vars_to_map(&build_cfg)
        .context("failed to read secrets/dev/build.cfg")?;

    let (port_key, not_running_hint) = if env == "dev" {
        (
            "ANT_MATCHMAKER_HTTP_PORT",
            "run `ah dev ant-matchmaker` first".to_string(),
        )
    } else {
        (
            "ANT_MATCHMAKER_CLIENT_HTTP_PORT",
            format!("run `ah dev ant-matchmaker client` first to connect to the {env} cluster"),
        )
    };

    let consul_port: u16 = vars
        .get(port_key)
        .with_context(|| format!("{port_key} not set in build.cfg"))?
        .parse()?;

    let sd = ServiceDiscovery::new(consul_port);
    let endpoint = sd.resolve(service).await.ok_or_else(|| {
        anyhow::anyhow!("service '{service}' not found in {env} Consul — {not_running_hint}")
    })?;

    Ok((endpoint.address, endpoint.port))
}
