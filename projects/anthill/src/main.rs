use anyhow::Result;
use clap::Parser;

mod build;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    Build(build::BuildCmd),
}

#[tokio::main(flavor = "local")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build(cmd)) => {
            build::build(cmd).await;
        }
        None => {}
    }

    Ok(())
}
