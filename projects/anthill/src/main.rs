use anyhow::Result;
use clap::{CommandFactory, Parser};

mod build;
mod complete;
mod dev;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
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

    match cli.command {
        Some(Commands::Build(cmd)) => {
            build::build(cmd).await;
        }
        Some(Commands::Dev(cmd)) => {
            dev::dev(cmd).await?;
        }
        None => {}
    }

    Ok(())
}
