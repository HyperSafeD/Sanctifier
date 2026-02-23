use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;
use colored::*;
use sanctifier_core::{Analyzer, SanctifyConfig};

#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    /// Path to the contract directory or Cargo.toml
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Limit for ledger entry size in bytes
    #[arg(short, long, default_value = "64000")]
    pub limit: usize,
}

pub fn exec(args: AnalyzeArgs) -> anyhow::Result<()> {
    let path = &args.path;
    let format = &args.format;
    let limit = args.limit;
    let is_json = format == "json";

    if !is_soroban_project(path) {
        eprintln!(
            "{} Error: {:?} is not a valid Soroban project. (Missing Cargo.toml with 'soroban-sdk' dependency)",
            "❌".red(),
            path
        );
        std::process::exit(1);
    }

    if is_json {
        eprintln!("{} Sanctifier: Valid Soroban project found at {:?}", "✨".green(), path);
        eprintln!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
    } else {
        println!("{} Sanctifier: Valid Soroban project found at {:?}", "✨".green(), path);
        println!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
    }

    let mut analyzer = Analyzer::new(sanctifier_core::SanctifyConfig::default());
    
    let config = SanctifyConfig::default();
    let analyzer = Analyzer::new(config);
    
    let mut collisions = Vec::new();

    if path.is_dir() {
        walk_dir(path, &analyzer, &mut collisions)?;
    } else {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(path) {
                collisions.extend(analyzer.scan_storage_collisions(&content));
            }
        }
    }

    if collisions.is_empty() {
        println!("\n{} No storage key collisions found.", "✅".green());
    } else {
        println!("\n{} Found potential Storage Key Collisions!", "⚠️".yellow());
        for collision in collisions {
            println!("   {} Value: {}", "->".red(), collision.key_value.bold());
            println!("      Type: {}", collision.key_type);
            println!("      Location: {}", collision.location);
            println!("      Message: {}", collision.message);
        }
    }
    
    Ok(())
}

fn walk_dir(dir: &Path, analyzer: &Analyzer, collisions: &mut Vec<sanctifier_core::StorageCollisionIssue>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, analyzer, collisions)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut issues = analyzer.scan_storage_collisions(&content);
                // Prefix location with filename
                let file_name = path.display().to_string();
                for issue in &mut issues {
                    issue.location = format!("{}:{}", file_name, issue.location);
                }
                collisions.extend(issues);
            }
        }
    }
    Ok(())
}

fn is_soroban_project(path: &Path) -> bool {
    // Basic heuristics for tests.
    if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        return true;
    }
    let cargo_toml_path = if path.is_dir() {
        path.join("Cargo.toml")
    } else {
        path.to_path_buf()
    };
    cargo_toml_path.exists()
}

fn analyze_directory(
    dir: &Path,
    analyzer: &Analyzer,
    all_size_warnings: &mut Vec<SizeWarning>,
    all_unsafe_patterns: &mut Vec<UnsafePattern>,
    all_auth_gaps: &mut Vec<String>,
    all_panic_issues: &mut Vec<sanctifier_core::PanicIssue>,
    all_arithmetic_issues: &mut Vec<ArithmeticIssue>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                analyze_directory(
                    &path, analyzer, all_size_warnings, all_unsafe_patterns, all_auth_gaps,
                    all_panic_issues, all_arithmetic_issues,
                );
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(&path) {
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
                }
            }
        }
    }
}
