use anyhow::Result;
use clap::{CommandFactory, Parser};

mod build;
mod complete;
mod dev;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
enum Cli {
    Build(build::BuildCmd),
    Dev(dev::DevCmd),
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
            build::build(cmd).await;
        }
        Cli::Dev(cmd) => {
            dev::dev(cmd).await?;
        }
    }

    Ok(())
}
