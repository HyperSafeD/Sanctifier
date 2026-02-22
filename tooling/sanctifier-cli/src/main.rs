use clap::{Parser, Subcommand};
mod commands;

use commands::analyze::AnalyzeArgs;

#[derive(Parser)]
#[command(name = "sanctifier")]
#[command(about = "Stellar Soroban Security & Formal Verification Suite", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a Soroban contract for vulnerabilities
    Analyze(AnalyzeArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze(args) => {
            commands::analyze::exec(args)?;
        }
    }

    Ok(())
}
