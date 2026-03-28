#![recursion_limit = "512"]

use clap::{Parser, Subcommand};
use colored::*;
use sanctifier_core::{callgraph_to_dot, Analyzer, SanctifyConfig};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::error;

mod commands;
mod logging;
pub mod vulndb;

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
    /// Generate a dynamic Sanctifier status badge
    Badge(commands::badge::BadgeArgs),
    /// Generate a Markdown or HTML security report
    Report(commands::report::ReportArgs),
    /// Initialize Sanctifier in a new project
    Init(commands::init::InitArgs),
    /// Show per-contract complexity metrics (cyclomatic complexity, nesting, LOC)
    Complexity(commands::complexity::ComplexityArgs),
    /// Generate a Graphviz DOT call graph of cross-contract calls (env.invoke_contract)
    Callgraph {
        /// Path to a contract directory, workspace directory, or a single .rs file
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output DOT file path
        #[arg(short, long, default_value = "callgraph.dot")]
        output: PathBuf,
    },
    /// Check for and download the latest Sanctifier binary
    Update,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(2);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let log_output = match &cli.command {
        Commands::Analyze(args) if args.format == "json" => logging::LogOutput::Json,
        _ => logging::LogOutput::Text,
    };
    logging::init(log_output)?;

    match cli.command {
        Commands::Analyze(args) => commands::analyze::exec(args)?,
        Commands::Badge(args) => {
            commands::badge::exec(args)?;
        }
        Commands::Complexity(args) => {
            commands::complexity::exec(args)?;
        }
        Commands::Report(args) => {
            commands::report::exec(args)?;
        }
        Commands::Init(args) => {
            let path = Some(args.path.clone());
            commands::init::exec(args, path)?;
        }
        Commands::Callgraph { path, output } => {
            let config = load_config(&path);
            let analyzer = Analyzer::new(config.clone());

            let mut rs_files: Vec<PathBuf> = Vec::new();
            if path.is_dir() {
                collect_rs_files(&path, &config, &mut rs_files);
            } else {
                rs_files.push(path.clone());
            }

            let mut edges = Vec::new();
            for f in rs_files {
                if f.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }

                let content = match fs::read_to_string(&f) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let caller = infer_contract_name(&content).unwrap_or_else(|| {
                    f.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("<unknown>")
                        .to_string()
                });

                let file_label = f.display().to_string();
                edges.extend(analyzer.scan_invoke_contract_calls(&content, &caller, &file_label));
            }

            let dot = callgraph_to_dot(&edges);
            if let Err(e) = fs::write(&output, dot) {
                error!(
                    target: "sanctifier",
                    output = %output.display(),
                    error = %e,
                    "Failed to write DOT file"
                );
                std::process::exit(1);
            }
            println!(
                "{} Wrote call graph to {:?} ({} edges)",
                "✅".green(),
                output,
                edges.len()
            );
        }
        Commands::Update => {
            commands::update::exec()?;
        }
    }

    Ok(())
}

fn collect_rs_files(dir: &Path, config: &SanctifyConfig, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if config.ignore_paths.iter().any(|p| name.contains(p)) {
                continue;
            }
            collect_rs_files(&path, config, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn infer_contract_name(source: &str) -> Option<String> {
    let mut saw_contract_attr = false;
    for line in source.lines() {
        let l = line.trim();
        if l.starts_with("#[contract]") {
            saw_contract_attr = true;
            continue;
        }
        if saw_contract_attr {
            if let Some(rest) = l.strip_prefix("pub struct ") {
                return Some(
                    rest.trim_end_matches(';')
                        .split_whitespace()
                        .next()?
                        .to_string(),
                );
            }
            if let Some(rest) = l.strip_prefix("struct ") {
                return Some(
                    rest.trim_end_matches(';')
                        .split_whitespace()
                        .next()?
                        .to_string(),
                );
            }
        }
    }
    None
}

fn load_config(path: &Path) -> SanctifyConfig {
    let mut current = if path.is_file() {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path.to_path_buf()
    };

    loop {
        let config_path = current.join(".sanctify.toml");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!(
                            "Error: Found .sanctify.toml at {} but it could not be parsed:\n  {}\n\
                             \n\
                             Run 'sanctifier init' to regenerate a valid config, or check the schema at:\n\
                             https://github.com/HyperSafeD/Sanctifier/blob/main/schemas/sanctify-config.schema.json",
                            config_path.display(),
                            e
                        );
                        std::process::exit(1);
                    }
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    SanctifyConfig::default()
}
