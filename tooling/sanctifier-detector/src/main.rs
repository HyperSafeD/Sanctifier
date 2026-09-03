use anyhow::Result;
use clap::Parser;
use sanctifier_detector::{DetectorConfig, DetectorService};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "sanctifier-detector",
    about = "Poll recorded call events and alert on anomalies"
)]
struct Args {
    #[arg(long)]
    config: PathBuf,

    #[arg(long)]
    once: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let config = DetectorConfig::load(&args.config)?;
    let mut service = DetectorService::new(config)?;

    if args.once {
        service.poll_once()?;
    } else {
        service.run()?;
    }

    Ok(())
}
