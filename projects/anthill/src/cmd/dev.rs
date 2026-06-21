use anthill_manifest::AnthillManifest;
use anyhow::Context;
use clap_complete::engine::ArgValueCompleter;

use crate::complete::complete_projects;
use crate::git::GitState;
use crate::procs;

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

    procs::spawn_and_wait(&project_dir, &manifest, &env, &cmd.args).await
}
