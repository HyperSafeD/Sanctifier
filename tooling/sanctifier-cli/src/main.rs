#![recursion_limit = "512"]

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;

use sanctifier_cli::commands;
use sanctifier_cli::logging;

#[derive(Parser)]
#[command(
    name = "sanctifier",
    version,
    about = "Soroban smart contract security analyzer"
)]
struct Cli {
    /// Disable coloured output (also respects NO_COLOR env var)
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Opt-in to anonymous telemetry reporting for this invocation.
    /// Not global: the `init` subcommand has its own `--telemetry <mode>` option.
    #[arg(long)]
    pub telemetry: bool,

    /// Target network (testnet, futurenet, mainnet).
    /// Overrides SOROBAN_NETWORK env var. Default: testnet.
    #[arg(short = 'n', long, global = true, default_value = "testnet")]
    pub network: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze a Soroban contract for vulnerabilities
    Analyze(commands::analyze::AnalyzeArgs),
    /// Initialize a .sanctify.toml configuration file
    Init(commands::init::InitArgs),
    /// Language Server Protocol (LSP) for editor integration
    Lsp(commands::lsp::LspArgs),
    /// Generate a security report
    Report(commands::report::ReportArgs),
    /// Estimate gas / instruction costs for a contract source file or workspace
    Gas(commands::gas::GasArgs),
    /// Detect potential storage key collisions in Soroban contracts
    Storage(commands::storage::StorageArgs),
    /// Install git hooks (pre-commit, pre-push) to run Sanctifier automatically
    InstallHooks(commands::install_hooks::InstallHooksArgs),
    /// Show per-contract complexity metrics (cyclomatic complexity, nesting, LOC)
    Complexity(commands::complexity::ComplexityArgs),
    /// Apply auto-fix patches to a contract; use --interactive to review each patch
    Fix(commands::fix::FixArgs),
    /// Explain a finding code (e.g. S001, S003) with details and remediation
    Explain(commands::explain::ExplainArgs),
    /// Check for and download the latest Sanctifier binary
    Update {
        /// Report what would be updated without invoking `cargo install`
        #[arg(long)]
        dry_run: bool,
    },
    /// Self-update with checksum verification via GitHub Releases
    Upgrade(commands::upgrade::UpgradeArgs),
    /// Detect reentrancy vulnerabilities (state mutation before external call)
    Reentrancy(commands::reentrancy::ReentrancyArgs),
    /// Verify local source against on-chain bytecode
    Verify(commands::verify::VerifyArgs),
    /// Verify an on-chain deployment matches expected local source or a pinned hash
    VerifyDeployment(commands::verify_deployment::VerifyDeploymentArgs),
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
    /// Generate a DOT call-graph of cross-contract invoke_contract calls
    Callgraph(commands::callgraph::CallgraphArgs),
    /// Run environment sanity checks (rustc, soroban-cli, z3, cargo-expand)
    Doctor(commands::doctor::DoctorArgs),
    /// Export analysis findings to CSV
    Export(commands::export::ExportArgs),
    /// Generate a security badge SVG from a Sanctifier JSON report
    Badge(commands::badge::BadgeArgs),
    /// Compare two scan results and show new/resolved findings
    Diff(commands::diff::DiffArgs),
    /// Generate native AFL/honggfuzz fuzz-harness scaffolds from a contract's ABI
    Harness(commands::harness::HarnessArgs),
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(sanctifier_cli::exit_codes::ERROR);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        commands::color::set_no_color(true);
    }

    // Print network indicator banner
    let network_badge = match cli.network.as_str() {
        "mainnet" => format!("{}", commands::color::red_bold("[ MAINNET ]")),
        "futurenet" => format!("{}", commands::color::yellow_bold("[ FUTURENET ]")),
        _ => format!("{}", commands::color::green_bold("[ TESTNET ]")),
    };
    eprintln!(
        "{} Sanctifier — {}",
        network_badge,
        commands::color::dimmed(&cli.network)
    );

    // Initialize structured logging before dispatching
    let log_format = match &cli.command {
        Commands::Analyze(args) if args.format == "json" || args.format == "sarif" => {
            logging::LogOutput::Json
        }
        _ => logging::LogOutput::Text,
    };
    if let Err(e) = logging::init(log_format) {
        eprintln!("Warning: failed to init logging: {e}");
    }

    // Print network indicator banner (suppressed in JSON log mode so stderr
    // stays machine-parseable).
    if log_format != logging::LogOutput::Json {
        let network_badge = match cli.network.as_str() {
            "mainnet" => commands::color::red_bold("[ MAINNET ]").to_string(),
            "futurenet" => commands::color::yellow_bold("[ FUTURENET ]").to_string(),
            _ => commands::color::green_bold("[ TESTNET ]").to_string(),
        };
        eprintln!(
            "{} Sanctifier — {}",
            network_badge,
            commands::color::dimmed(&cli.network)
        );
    }

    match cli.command {
        Commands::Analyze(args) => commands::analyze::exec(args),
        Commands::Init(args) => commands::init::exec(args, None),
        Commands::Lsp(args) => commands::lsp::exec(args),
        Commands::Report(args) => commands::report::exec(args),
        Commands::Gas(args) => commands::gas::exec(args),
        Commands::Storage(args) => commands::storage::exec(args),
        Commands::InstallHooks(args) => commands::install_hooks::exec(args),
        Commands::Complexity(args) => commands::complexity::exec(args),
        Commands::Fix(args) => commands::fix::exec(args),
        Commands::Explain(args) => commands::explain::exec(args),
        Commands::Update { dry_run } => commands::update::exec(dry_run),
        Commands::Upgrade(args) => commands::upgrade::exec(args),
        Commands::Reentrancy(args) => commands::reentrancy::exec(args),
        Commands::Verify(args) => commands::verify::exec(args),
        Commands::VerifyDeployment(args) => commands::verify_deployment::exec(args),
        Commands::Workspace(args) => commands::workspace::exec(args),
        Commands::Watch(args) => commands::watch::exec(args),
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "sanctifier", &mut io::stdout());
            Ok(())
        }
        Commands::Suppress(args) => commands::suppress::exec(args),
        Commands::Serve(args) => commands::serve::exec(args),
        Commands::Benchmark(args) => commands::benchmark::exec(args),
        Commands::Callgraph(args) => commands::callgraph::exec(args),
        Commands::Doctor(args) => commands::doctor::exec(args),
        Commands::Export(args) => commands::export::exec(args),
        Commands::Badge(args) => commands::badge::exec(args),
        Commands::Diff(args) => commands::diff::exec(args),
        Commands::Harness(args) => commands::harness::exec(args),
    }
}
