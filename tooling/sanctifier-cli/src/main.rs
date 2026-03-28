use clap::{Parser, Subcommand};
mod branding;
mod commands;

#[derive(Parser)]
#[command(name = "sanctifier")]
#[command(about = "Stellar Soroban Security & Formal Verification Suite", long_about = None)]
#[command(version = concat!(
    env!("CARGO_PKG_VERSION"),
    "\nCommit: ", env!("VERGEN_GIT_SHA"), " (", env!("VERGEN_GIT_COMMIT_DATE"), ")",
    "\nBuild date: ", env!("VERGEN_BUILD_DATE"),
    "\nTarget: ", env!("VERGEN_CARGO_TARGET_TRIPLE"),
    "\nBuilt with: rustc ", env!("VERGEN_RUSTC_SEMVER")
))]

struct Cli {
    #[command(subcommand)]
    command: Commands,
}



#[derive(Subcommand)]
pub enum Commands {
    /// Analyze a Soroban contract for vulnerabilities
    Analyze(commands::analyze::AnalyzeArgs),
    /// Generate a security report
    Report {
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Initialize Sanctifier in a new project
    Init(commands::init::InitArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze(args) => {
            if args.format != "json" {
                branding::print_logo();
            }
            commands::analyze::exec(args)?;
        }
        Commands::Report { output } => {
            if let Some(p) = output {
                println!("Report saved to {:?}", p);
            } else {
                println!("Report printed to stdout.");
            }
        }
        Commands::Init(args) => {
            commands::init::exec(args, None)?;
        }
    }



    Ok(())
}
