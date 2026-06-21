use anyhow::Result;
use clap::{CommandFactory, Parser};

mod cmd;
mod complete;
mod git;
mod procs;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
enum Cli {
    Build(crate::cmd::build::BuildCmd),
    Deploy(crate::cmd::deploy::DeployCmd),
    Dev(crate::cmd::dev::DevCmd),
    Run(crate::cmd::run::RunCmd),
    Curl(crate::cmd::curl::CurlCmd),
}

#[tokio::main(flavor = "local")]
async fn main() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command)
        .bin("ah")
        .complete();

    ant_library::set_global_logs("anthill");

    let cli = Cli::parse();

    match cli {
        Cli::Build(cmd) => {
            crate::cmd::build::build(cmd).await;
        }
        Cli::Deploy(cmd) => {
            crate::cmd::deploy::deploy(cmd).await;
        }
        Cli::Dev(cmd) => {
            crate::cmd::dev::dev(cmd).await?;
        }
        Cli::Run(cmd) => {
            crate::cmd::run::run(cmd).await?;
        }
        Cli::Curl(cmd) => {
            crate::cmd::curl::curl(cmd).await?;
        }
    }

    Ok(())
}
