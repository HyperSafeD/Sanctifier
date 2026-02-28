use clap::{Parser, Subcommand};
use colored::*;
use serde::Deserialize;
use sanctifier_core::{callgraph_to_dot, Analyzer, ArithmeticIssue, CustomRuleMatch, SanctifyConfig, SizeWarning, UnsafePattern, UpgradeReport};
use std::fs;
use std::path::{Path, PathBuf};
mod branding;
mod commands;
pub mod vulndb;

#[derive(Parser)]
#[command(name = "sanctifier")]
#[command(about = "Stellar Soroban Security & Formal Verification Suite", long_about = None)]
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
    /// Generate a security report
    Report {
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Initialize Sanctifier in a new project
    Init,

    /// Generate a Graphviz DOT call graph of cross-contract calls (env.invoke_contract)
    Callgraph {
        /// Path to a contract directory, workspace directory, or a single .rs file
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output DOT file path
        #[arg(short, long, default_value = "callgraph.dot")]
        output: PathBuf,
    },
    Init(commands::init::InitArgs),
    /// Check for and download the latest Sanctifier binary
    Update,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Analyze {
            path,
            format,
            limit,
        } => {
            let is_json = format == "json";

            if !is_soroban_project(path) {
                eprintln!("{} Error: {:?} is not a valid Soroban project. (Missing Cargo.toml with 'soroban-sdk' dependency)", "❌".red(), path);
                std::process::exit(1);
            }

            // In JSON mode, send informational lines to stderr so stdout is clean JSON.
            if is_json {
                eprintln!(
                    "{} Sanctifier: Valid Soroban project found at {:?}",
                    "✨".green(),
                    path
                );
                eprintln!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
            } else {
                println!(
                    "{} Sanctifier: Valid Soroban project found at {:?}",
                    "✨".green(),
                    path
                );
                println!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
            }

            let mut config = load_config(path);
            config.ledger_limit = *limit;

            let analyzer = Analyzer::new(config.clone());

            let mut all_size_warnings: Vec<SizeWarning> = Vec::new();
            let mut all_unsafe_patterns: Vec<UnsafePattern> = Vec::new();
            let mut all_auth_gaps: Vec<String> = Vec::new();
            let mut all_panic_issues: Vec<sanctifier_core::PanicIssue> = Vec::new();
            let mut all_arithmetic_issues: Vec<ArithmeticIssue> = Vec::new();
            let mut all_custom_rule_matches: Vec<CustomRuleMatch> = Vec::new();
            let mut upgrade_report = UpgradeReport::empty();

            if path.is_dir() {
                analyze_directory(
                    path,
                    &analyzer,
                    &config,
                    &mut all_size_warnings,
                    &mut all_unsafe_patterns,
                    &mut all_auth_gaps,
                    &mut all_panic_issues,
                    &mut all_arithmetic_issues,
                    &mut all_custom_rule_matches,
                    &mut upgrade_report,
                );
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(path) {
                    all_size_warnings.extend(analyzer.analyze_ledger_size(&content));

                    let patterns = analyzer.analyze_unsafe_patterns(&content);
                    for mut p in patterns {
                        p.snippet = format!("{}: {}", path.display(), p.snippet);
                        all_unsafe_patterns.push(p);
                    }

                    let gaps = analyzer.scan_auth_gaps(&content);
                    for g in gaps {
                        all_auth_gaps.push(format!("{}: {}", path.display(), g));
                    }

                    let panics = analyzer.scan_panics(&content);
                    for p in panics {
                        let mut p_mod = p.clone();
                        p_mod.location = format!("{}: {}", path.display(), p.location);
                        all_panic_issues.push(p_mod);
                    }

                    let arith = analyzer.scan_arithmetic_overflow(&content);
                    for mut a in arith {
                        a.location = format!("{}: {}", path.display(), a.location);
                        all_arithmetic_issues.push(a);
                    }

                    let custom_matches = analyzer.analyze_custom_rules(&content, &config.custom_rules);
                    for mut m in custom_matches {
                        m.snippet = format!("{}: {}", path.display(), m.snippet);
                        all_custom_rule_matches.push(m);
                    }
                }
            }

            if is_json {
                eprintln!("{} Static analysis complete.", "✅".green());
            } else {
                println!("{} Static analysis complete.", "✅".green());
            }

            if format == "json" {
                let output = serde_json::json!({
                    "size_warnings": all_size_warnings,
                    "unsafe_patterns": all_unsafe_patterns,
                    "auth_gaps": all_auth_gaps,
                    "panic_issues": all_panic_issues,
                    "arithmetic_issues": all_arithmetic_issues,
                    "custom_rule_matches": all_custom_rule_matches,
                    "upgrade_report": upgrade_report,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                if !all_size_warnings.is_empty() {
                    println!("\n{} Found Ledger Size Warnings!", "⚠️".yellow());
                    for warning in &all_size_warnings {
                        let (icon, msg) = match warning.level {
                            sanctifier_core::SizeWarningLevel::ExceedsLimit => ("🛑".red(), "exceeds"),
                            sanctifier_core::SizeWarningLevel::ApproachingLimit => ("⚠️".yellow(), "is approaching"),
                        };
                        println!(
                            "   {} {} {} the ledger entry size limit!",
                            icon,
                            warning.struct_name.bold(),
                            msg
                        );
                        println!(
                            "      Estimated size: {} bytes (Limit: {} bytes)",
                            warning.estimated_size.to_string().red(),
                            warning.limit
                        );
                    }
                } else {
                    println!("\nNo ledger size issues found.");
                }

                if !all_auth_gaps.is_empty() {
                    println!("\n{} Found potential Authentication Gaps!", "🛑".red());
                    for gap in all_auth_gaps {
                        println!(
                            "   {} Function {} is modifying state without require_auth()",
                            "->".red(),
                            gap.bold()
                        );
                    }
                } else {
                    println!("\nNo authentication gaps found.");
                }

                if !all_panic_issues.is_empty() {
                    println!("\n{} Found explicit Panics/Unwraps!", "🛑".red());
                    for issue in all_panic_issues {
                        println!(
                            "   {} Function {}: Using {} (Location: {})",
                            "->".red(),
                            issue.function_name.bold(),
                            issue.issue_type.yellow().bold(),
                            issue.location
                        );
                    }
                    println!("   {} Tip: Prefer returning Result or Error types for better contract safety.", "💡".blue());
                } else {
                    println!("\nNo panic/unwrap issues found.");
                }

                if !all_arithmetic_issues.is_empty() {
                    println!("\n{} Found unchecked Arithmetic Operations!", "🔢".yellow());
                    for issue in all_arithmetic_issues {
                        println!(
                            "   {} Function {}: Unchecked `{}` ({})",
                            "->".red(),
                            issue.function_name.bold(),
                            issue.operation.yellow().bold(),
                            issue.location
                        );
                        println!("      {} {}", "💡".blue(), issue.suggestion);
                    }
                } else {
                    println!("\nNo arithmetic overflow risks found.");
                }

                if !all_custom_rule_matches.is_empty() {
                    println!("\n{} Found Custom Rule Matches!", "📜".yellow());
                    for m in all_custom_rule_matches {
                        println!(
                            "   {} Rule {}: `{}` (Line: {})",
                            "->".yellow(),
                            m.rule_name.bold(),
                            m.snippet.trim().italic(),
                            m.line
                        );
                    }
                }

                if !upgrade_report.findings.is_empty()
                    || !upgrade_report.upgrade_mechanisms.is_empty()
                    || !upgrade_report.init_functions.is_empty()
                {
                    println!("\n{} Upgrade Pattern Analysis", "🔄".yellow());
                    for f in &upgrade_report.findings {
                        println!(
                            "   {} [{}] {} ({})",
                            "->".yellow(),
                            format!("{:?}", f.category).to_lowercase(),
                            f.message,
                            f.location
                        );
                        println!("      {} {}", "💡".blue(), f.suggestion);
                    }
                    if !upgrade_report.suggestions.is_empty() {
                        for s in &upgrade_report.suggestions {
                            println!("   {} {}", "💡".blue(), s);
                        }
                    }
                } else {
                    println!("\nNo upgrade pattern issues found.");
                }
    match cli.command {
        Commands::Analyze(args) => {
            if args.format != "json" {
                branding::print_logo();
            }
            commands::analyze::exec(args)?;
        }
        Commands::Badge(args) => {
            commands::badge::exec(args)?;
        }
        Commands::Report { output } => {
            if let Some(p) = output {
                println!("Report saved to {:?}", p);
            } else {
                println!("Report printed to stdout.");
            }
        }
        Commands::Init => {}

        Commands::Callgraph { path, output } => {
            let config = load_config(path);
            let analyzer = Analyzer::new(config.clone());

            let mut rs_files: Vec<PathBuf> = Vec::new();
            if path.is_dir() {
                collect_rs_files(path, &config, &mut rs_files);
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

                let caller = infer_contract_name(&content)
                    .unwrap_or_else(|| f.file_stem().and_then(|s| s.to_str()).unwrap_or("<unknown>").to_string());

                let file_label = f.display().to_string();
                edges.extend(analyzer.scan_invoke_contract_calls(&content, &caller, &file_label));
            }

            let dot = callgraph_to_dot(&edges);
            if let Err(e) = fs::write(output, dot) {
                eprintln!("{} Failed to write DOT file: {}", "❌".red(), e);
                std::process::exit(1);
            }
            println!("{} Wrote call graph to {:?} ({} edges)", "✅".green(), output, edges.len());
        }
    }
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
    // Heuristic: Soroban contracts commonly contain `#[contract]` then `pub struct Name;`
    let mut saw_contract_attr = false;
    for line in source.lines() {
        let l = line.trim();
        if l.starts_with("#[contract]") {
            saw_contract_attr = true;
            continue;
        }
        if saw_contract_attr {
            // Accept `pub struct X;` or `struct X;`
            if let Some(rest) = l.strip_prefix("pub struct ") {
                return Some(rest.trim_end_matches(';').split_whitespace().next()?.to_string());
            }
            if let Some(rest) = l.strip_prefix("struct ") {
                return Some(rest.trim_end_matches(';').split_whitespace().next()?.to_string());
            }
        }
    }
    None
}

fn is_soroban_project(path: &Path) -> bool {
    let cargo_toml_path = if path.is_dir() {
        path.join("Cargo.toml")
    } else if path.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml") {
        path.to_path_buf()
    } else {
        // If it's a .rs file, look for Cargo.toml in parent directories
        let mut current = path.parent();
        let mut found = None;
        while let Some(p) = current {
            let cargo = p.join("Cargo.toml");
            if cargo.exists() {
                found = Some(cargo);
                break;
            }
            current = p.parent();
        }
        match found {
            Some(f) => f,
            None => return false,
        }
    };

    if !cargo_toml_path.exists() {
        return false;
    }

    if let Ok(content) = fs::read_to_string(cargo_toml_path) {
        content.contains("soroban-sdk")
    } else {
        false
    }
}

fn analyze_directory(
    dir: &Path,
    analyzer: &Analyzer,
    config: &SanctifyConfig,
    all_size_warnings: &mut Vec<SizeWarning>,
    all_unsafe_patterns: &mut Vec<UnsafePattern>,
    all_auth_gaps: &mut Vec<String>,
    all_panic_issues: &mut Vec<sanctifier_core::PanicIssue>,
    all_arithmetic_issues: &mut Vec<ArithmeticIssue>,
    all_custom_rule_matches: &mut Vec<CustomRuleMatch>,
    upgrade_report: &mut UpgradeReport,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if config.ignore_paths.iter().any(|p| name.contains(p)) {
                    continue;
                }
                analyze_directory(
                    &path,
                    analyzer,
                    config,
                    all_size_warnings,
                    all_unsafe_patterns,
                    all_auth_gaps,
                    all_panic_issues,
                    all_arithmetic_issues,
                    all_custom_rule_matches,
                    upgrade_report,
                );
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let warnings = analyzer.analyze_ledger_size(&content);
                    for mut w in warnings {
                        w.struct_name = format!("{}: {}", path.display(), w.struct_name);
                        all_size_warnings.push(w);
                    }

                    let patterns = analyzer.analyze_unsafe_patterns(&content);
                    for mut p in patterns {
                        p.snippet = format!("{}: {}", path.display(), p.snippet);
                        all_unsafe_patterns.push(p);
                    }

                    let gaps = analyzer.scan_auth_gaps(&content);
                    for g in gaps {
                        all_auth_gaps.push(format!("{}: {}", path.display(), g));
                    }

                    let panics = analyzer.scan_panics(&content);
                    for p in panics {
                        let mut p_mod = p.clone();
                        p_mod.location = format!("{}: {}", path.display(), p.location);
                        all_panic_issues.push(p_mod);
                    }

                    let arith = analyzer.scan_arithmetic_overflow(&content);
                    for mut a in arith {
                        a.location = format!("{}: {}", path.display(), a.location);
                        all_arithmetic_issues.push(a);
                    }

                    let custom_matches = analyzer.analyze_custom_rules(&content, &config.custom_rules);
                    for mut m in custom_matches {
                        m.snippet = format!("{}: {}", path.display(), m.snippet);
                        all_custom_rule_matches.push(m);
                    }
                }
            }
        Commands::Init(args) => {
            commands::init::exec(args, None)?;
        }
        Commands::Update => {
            commands::update::exec()?;
        }
    }

    Ok(())
}
