#![recursion_limit = "512"]

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use sanctifier_core::SanctifyConfig;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::error;

mod commands;
mod logging;
mod telemetry;
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
    /// Disable coloured output (also respects NO_COLOR env var)
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze a Soroban contract for vulnerabilities
    Analyze(commands::analyze::AnalyzeArgs),
    /// Compare current scan results against a baseline to find only NEW vulnerabilities
    Diff(commands::diff::DiffArgs),
    /// Generate a dynamic Sanctifier status badge
    Badge(commands::badge::BadgeArgs),
    /// Generate a Markdown or HTML security report
    Report(commands::report::ReportArgs),
    /// Estimate gas / instruction costs for a contract source file or workspace
    Gas(commands::gas::GasArgs),
    /// Detect potential storage key collisions in Soroban contracts
    Storage(commands::storage::StorageArgs),
    /// Initialize Sanctifier in a new project
    Init(commands::init::InitArgs),
    /// Install git hooks (pre-commit, pre-push) to run Sanctifier automatically
    InstallHooks(commands::install_hooks::InstallHooksArgs),
    /// Show per-contract complexity metrics (cyclomatic complexity, nesting, LOC)
    Complexity(commands::complexity::ComplexityArgs),
    /// Generate a Graphviz DOT call graph of cross-contract calls (env.invoke_contract)
    Callgraph {
        /// Path to a contract directory, workspace directory, or a single .rs file
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: text | json | junit
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output DOT file path
        #[arg(short, long, default_value = "callgraph.dot")]
        output: PathBuf,
    },
    /// Apply auto-fix patches to a contract; use --interactive to review each patch
    Fix(commands::fix::FixArgs),
    /// Explain a finding code (e.g. S001, S003) with details and remediation
    Explain(commands::explain::ExplainArgs),
    /// Check for and download the latest Sanctifier binary
    Update,
    /// Self-update with checksum verification via GitHub Releases
    Upgrade(commands::upgrade::UpgradeArgs),
    /// Detect reentrancy vulnerabilities (state mutation before external call)
    Reentrancy(commands::reentrancy::ReentrancyArgs),
    /// Verify local source against on-chain bytecode
    Verify(commands::verify::VerifyArgs),
    /// Analyze an entire Cargo workspace (multiple contracts/libs)
    Workspace(commands::workspace::WorkspaceArgs),
    /// Watch for file changes and auto-rerun analysis
    Watch(commands::watch::WatchArgs),
    /// Generate shell completions for bash, zsh, fish, powershell, or elvish
    Completions {
        /// Shell type: bash, zsh, fish, powershell, elvish
        #[arg(value_parser = clap::value_parser!(Shell))]
        shell: Shell,
    },
    /// Suppress a finding by adding it to .sanctify.toml
    Suppress(commands::suppress::SuppressArgs),
    /// Start HTTP server mode for CI integration
    Serve(commands::serve::ServeArgs),
    /// Run the analyser on a contract corpus and emit a per-rule performance table
    Benchmark(commands::benchmark::BenchmarkArgs),
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
        Commands::Analyze(args) if args.format == "json" || args.format == "ndjson" => logging::LogOutput::Json,
        Commands::Diff(args) if args.format == "json" => logging::LogOutput::Json,
        Commands::Gas(args) if args.format == commands::gas::OutputFormat::Json => {
            logging::LogOutput::Json
        }
        Commands::Storage(args) if args.format == commands::storage::OutputFormat::Json => {
            logging::LogOutput::Json
        }
        _ => logging::LogOutput::Text,
    };
    logging::init(log_output)?;
    commands::color::init(cli.no_color);

    match cli.command {
        Commands::Analyze(args) => commands::analyze::exec(args)?,
        Commands::Diff(args) => commands::diff::exec(args)?,
        Commands::Badge(args) => {
            commands::badge::exec(args)?;
        }
        Commands::Complexity(args) => {
            commands::complexity::exec(args)?;
        }
        Commands::Report(args) => {
            commands::report::exec(args)?;
        }
        Commands::Gas(args) => {
            commands::gas::exec(args)?;
        }
        Commands::Storage(args) => {
            commands::storage::exec(args)?;
        }
        Commands::Init(args) => {
            let path = Some(args.path.clone());
            commands::init::exec(args, path)?;
        }
        Commands::InstallHooks(args) => {
            commands::install_hooks::exec(args)?;
        }
        Commands::Callgraph {
            path,
            format,
            output,
        } => {
            use sanctifier_core::Analyzer;
            fn callgraph_to_dot(edges: &[sanctifier_core::ReentrancyEdge]) -> String {
                let mut dot = String::from("digraph callgraph {\n");
                for e in edges {
                    dot.push_str(&format!("  \"{}\" -> \"{}\";\n", e.caller_function, e.target_contract));
                }
                dot.push_str("}\n");
                dot
            }
            let config = load_config(&path);
            let analyzer = Analyzer::new(config.clone());
            let is_json = format == "json";

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

            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&edges).unwrap_or_else(|_| "[]".to_string())
                );
            } else {
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
                    "✅ Wrote call graph to {:?} ({} edges)",
                    output,
                    edges.len()
                );
            }
        }
        Commands::Fix(args) => {
            commands::fix::exec(args)?;
        }
        Commands::Explain(args) => {
            commands::explain::exec(args)?;
        }
        Commands::Update => {
            commands::update::exec()?;
        }
        Commands::Upgrade(args) => {
            commands::upgrade::exec(args)?;
        }
        Commands::Reentrancy(args) => {
            commands::reentrancy::exec(args)?;
        }
        Commands::Verify(args) => {
            commands::verify::exec(args)?;
        }
        Commands::Workspace(args) => {
            commands::workspace::exec(args)?;
        }
        Commands::Watch(args) => {
            commands::watch::exec(args)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "sanctifier", &mut io::stdout());
        }
        Commands::Suppress(args) => {
            commands::suppress::exec(args)?;
        }
        Commands::Serve(args) => {
            commands::serve::exec(args)?;
        }
        Commands::Benchmark(args) => {
            commands::benchmark::exec(args)?;
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
