use clap::{Parser, Subcommand};
use colored::*;
use sanctifier_core::{Analyzer, ArithmeticIssue, CustomRuleMatch, SanctifyConfig, SizeWarning, UnsafePattern};
use std::fs;
use std::path::{Path, PathBuf};

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
    Analyze {
        /// Path to the contract directory or Cargo.toml
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Limit for ledger entry size in bytes
        #[arg(short, long, default_value = "64000")]
        limit: usize,
    },
    /// Generate a security report
    Report {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Initialize Sanctifier in a new project
    Init,
}

fn main() {
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
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                if all_size_warnings.is_empty() {
                    println!("\nNo ledger size issues found.");
                } else {
                    for warning in all_size_warnings {
                        println!(
                            "   {} Warning: Struct {} is approaching ledger entry size limit!",
                            "⚠️".yellow(),
                            warning.struct_name.bold()
                        );
                        println!(
                            "      Estimated size: {} bytes (Limit: {} bytes)",
                            warning.estimated_size.to_string().red(),
                            warning.limit
                        );
                    }
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
                } else {
                    println!("\nNo custom rule matches found.");
                }
            }
        }
        Commands::Report { output } => {
            println!("{} Generating report...", "📄".yellow());
            if let Some(p) = output {
                println!("Report saved to {:?}", p);
            } else {
                println!("Report printed to stdout.");
            }
        }
        Commands::Init => {}
    }
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
        }
    }
}

fn load_config(path: &Path) -> SanctifyConfig {
    find_config_path(path)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

fn find_config_path(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_dir() {
        Some(start_path.to_path_buf())
    } else {
        start_path.parent().map(|p| p.to_path_buf())
    };

    while let Some(path) = current {
        let config_path = path.join(".sanctify.toml");
        if config_path.exists() {
            return Some(config_path);
        }
        current = if path.parent().is_some() {
            path.parent().map(|p| p.to_path_buf())
        } else {
            None
        }
    }
    None
}
