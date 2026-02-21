use clap::{Parser, Subcommand};
use colored::*;
use std::path::{Path, PathBuf};
use std::fs;
use sanctifier_core::{Analyzer, SizeWarning, UnsafePattern, PatternType};

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
        Commands::Analyze { path, format, limit } => {
            println!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
            
            let mut analyzer = Analyzer::new(false);
            analyzer.ledger_limit = *limit;
            
            let mut all_size_warnings = Vec::new();
            let mut all_unsafe_patterns = Vec::new();

            println!("Debug: is_dir? {}, extension: {:?}", path.is_dir(), path.extension());
            if path.is_dir() {
                analyze_directory(path, &analyzer, &mut all_size_warnings, &mut all_unsafe_patterns);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                println!("Debug: Is a .rs file");
                if let Ok(content) = fs::read_to_string(path) {
                    let size_warnings = analyzer.analyze_ledger_size(&content);
                    let unsafe_patterns = analyzer.analyze_unsafe_patterns(&content);
                    println!("Found {} size warnings and {} unsafe patterns in {:?}", size_warnings.len(), unsafe_patterns.len(), path);
                    all_size_warnings.extend(size_warnings);
                    all_unsafe_patterns.extend(unsafe_patterns);
                } else {
                    println!("Debug: Failed to read file {:?}", path);
                }
            } else {
                println!("Debug: Path neither dir nor .rs file");
            }

            println!("{} Static analysis complete.", "✅".green());
            
            if format == "json" {
                let output = serde_json::json!({
                    "size_warnings": all_size_warnings,
                    "unsafe_patterns": all_unsafe_patterns,
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()));
            } else {
                if all_size_warnings.is_empty() && all_unsafe_patterns.is_empty() {
                    println!("No issues found.");
                } else {
                    for warning in all_size_warnings {
                        println!(
                            "{} Warning: Struct {} is approaching ledger entry size limit!",
                            "⚠️".yellow(),
                            warning.struct_name.bold()
                        );
                        println!(
                            "   Estimated size: {} bytes (Limit: {} bytes)",
                            warning.estimated_size.to_string().red(),
                            warning.limit
                        );
                    }

                    for pattern in all_unsafe_patterns {
                        let msg = match pattern.pattern_type {
                            PatternType::Panic => "Explicit panic!() call detected".red(),
                            PatternType::Unwrap => "unwrap() call detected".yellow(),
                            PatternType::Expect => "expect() call detected".yellow(),
                        };
                        println!(
                            "{} {}: {}",
                            "🚨".red(),
                            msg,
                            format!("{}:{}", pattern.line, pattern.snippet).bold()
                        );
                    }
                }
            }
        },
        Commands::Report { output } => {
            println!("{} Generating report...", "📄".yellow());
            if let Some(p) = output {
                println!("Report saved to {:?}", p);
            } else {
                println!("Report printed to stdout.");
            }
        },
        Commands::Init => {
            println!("{} Initializing Sanctifier configuration...", "⚙️".cyan());
            println!("Created .sanctify.toml");
        }
    }
}

fn analyze_directory(
    dir: &Path, 
    analyzer: &Analyzer, 
    all_size_warnings: &mut Vec<SizeWarning>,
    all_unsafe_patterns: &mut Vec<UnsafePattern>
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                analyze_directory(&path, analyzer, all_size_warnings, all_unsafe_patterns);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let warnings = analyzer.analyze_ledger_size(&content);
                    for mut w in warnings {
                        w.struct_name = format!("{}: {}", path.display(), w.struct_name);
                        all_size_warnings.push(w);
                    }

                    let patterns = analyzer.analyze_unsafe_patterns(&content);
                    for mut p in patterns {
                        p.snippet = format!("{}:{}", path.display(), p.snippet);
                        all_unsafe_patterns.push(p);
                    }
                }
            }
        }
    }
}
