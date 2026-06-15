use anthill_manifest::AnthillManifest;
use anyhow::Context;
use clap_complete::engine::ArgValueCompleter;

use crate::build::GitState;
use crate::complete::complete_projects;
use crate::procs;

#[derive(clap::Args)]
pub struct RunCmd {
    #[arg(add = ArgValueCompleter::new(complete_projects))]
    project: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

pub async fn run(cmd: RunCmd) -> Result<(), anyhow::Error> {
    let repo_root = GitState::new().context("failed to find git root")?.root;

    let project_dir = repo_root.join("projects").join(&cmd.project);
    let manifest = AnthillManifest::from_file(&project_dir.join("anthill.json"))
        .context(format!("no anthill.json for project {}", cmd.project))?;

    let build_cfg = repo_root
        .join("projects")
        .join("ant-zookeeper")
        .join("dev-fs")
        .join("dev-fs")
        .join("envs")
        .join("prod.build.cfg");
    anyhow::ensure!(
        build_cfg.exists(),
        "prod.build.cfg not found at {} — is ant-zookeeper checked out?",
        build_cfg.display()
    );

    let secrets_dir = repo_root
        .join("projects")
        .join("ant-zookeeper")
        .join("dev-fs")
        .join("dev-fs")
        .join("secrets-db")
        .join("prod");
    anyhow::ensure!(
        secrets_dir.exists(),
        "prod secrets not found at {} — provision them first",
        secrets_dir.display()
    );

    let mut env = ant_library::env::env_vars_to_map(&build_cfg)
        .context("failed to read prod.build.cfg")?;

    env.extend(manifest.to_port_env_vars(&cmd.project));
    env.insert(
        "TYPESOFANTS_SECRET_DIR".to_string(),
        secrets_dir.to_string_lossy().into_owned(),
    );
    env.insert(
        "PERSIST_DIR".to_string(),
        project_dir.join("dev-fs").to_string_lossy().into_owned(),
    );

    procs::spawn_and_wait(&project_dir, &manifest, &env, &cmd.args).await
}
