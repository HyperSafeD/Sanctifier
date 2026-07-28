use crate::commands::analyze::{self, AnalyzeArgs, AnalysisProfile, SeverityLevel};
use crate::commands::color as c;
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CiArgs {
    /// Path to the contract directory or Cargo.toml
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format (e.g. text, json, sarif)
    #[arg(short, long, default_value = "text")]
    pub format: String,
}

pub fn exec(args: CiArgs) -> anyhow::Result<()> {
    println!("{}", c::bold("sanctifier ci — running full analysis and verification gate"));
    println!();

    let analyze_args = AnalyzeArgs {
        path: args.path,
        format: args.format,
        limit: 64000,
        vuln_db: None,
        timeout: 30,
        webhook_urls: vec![],
        webhook_secret: None,
        exit_code: true,
        min_severity: SeverityLevel::High,
        no_cache: true, // A CI run should start fresh
        profile: Some(AnalysisProfile::Ci),
    };

    analyze::exec(analyze_args)
}
