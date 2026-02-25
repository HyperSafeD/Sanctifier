use clap::Args;
use colored::*;
use sanctifier_core::{Analyzer, SanctifyConfig};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

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
    let _limit = args.limit;
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

    let config = SanctifyConfig::default();
    let analyzer = Analyzer::new(config);

    let mut collisions = Vec::new();
    let mut size_warnings = Vec::new();
    let mut unsafe_patterns = Vec::new();
    let mut auth_gaps = Vec::new();
    let mut panic_issues = Vec::new();
    let mut arithmetic_issues = Vec::new();

    if path.is_dir() {
        walk_dir(
            path,
            &analyzer,
            &mut collisions,
            &mut size_warnings,
            &mut unsafe_patterns,
            &mut auth_gaps,
            &mut panic_issues,
            &mut arithmetic_issues,
        )?;
    } else {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(path) {
                collisions.extend(analyzer.scan_storage_collisions(&content));
                size_warnings.extend(analyzer.analyze_ledger_size(&content));
                unsafe_patterns.extend(analyzer.analyze_unsafe_patterns(&content));
                auth_gaps.extend(analyzer.scan_auth_gaps(&content));
                panic_issues.extend(analyzer.scan_panics(&content));
                arithmetic_issues.extend(analyzer.scan_arithmetic_overflow(&content));
            }
        }
    }

    if is_json {
        let report = serde_json::json!({
            "storage_collisions": collisions,
            "ledger_size_warnings": size_warnings,
            "unsafe_patterns": unsafe_patterns,
            "auth_gaps": auth_gaps,
            "panic_issues": panic_issues,
            "arithmetic_issues": arithmetic_issues,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if collisions.is_empty() {
        println!("\n{} No storage key collisions found.", "✅".green());
    } else {
        println!(
            "\n{} Found potential Storage Key Collisions!",
            "⚠️".yellow()
        );
        for collision in collisions {
            println!("   {} Value: {}", "->".red(), collision.key_value.bold());
            println!("      Type: {}", collision.key_type);
            println!("      Location: {}", collision.location);
            println!("      Message: {}", collision.message);
        }
    }

    if auth_gaps.is_empty() {
        println!("{} No authentication gaps found.", "✅".green());
    } else {
        println!("\n{} Found potential Authentication Gaps!", "⚠️".yellow());
        for gap in auth_gaps {
            println!("   {} Function: {}", "->".red(), gap.bold());
        }
    }

    if panic_issues.is_empty() {
        println!("{} No explicit Panics/Unwraps found.", "✅".green());
    } else {
        println!("\n{} Found explicit Panics/Unwraps!", "⚠️".yellow());
        for issue in panic_issues {
            println!("   {} Type: {}", "->".red(), issue.issue_type.bold());
            println!("      Location: {}", issue.location);
        }
    }

    if arithmetic_issues.is_empty() {
        println!("{} No unchecked Arithmetic Operations found.", "✅".green());
    } else {
        println!("\n{} Found unchecked Arithmetic Operations!", "⚠️".yellow());
        for issue in arithmetic_issues {
            println!("   {} Op: {}", "->".red(), issue.operation.bold());
            println!("      Location: {}", issue.location);
        }
    }

    if size_warnings.is_empty() {
        println!("{} No ledger size issues found.", "✅".green());
    } else {
        println!("\n{} Found Ledger Size Warnings!", "⚠️".yellow());
        for warning in size_warnings {
            println!("   {} Struct: {}", "->".red(), warning.struct_name.bold());
            println!("      Size: {} bytes", warning.estimated_size);
        }
    }

    println!("\n{} Static analysis complete.", "✨".green());

    Ok(())
}

fn walk_dir(
    dir: &Path,
    analyzer: &Analyzer,
    collisions: &mut Vec<sanctifier_core::StorageCollisionIssue>,
    size_warnings: &mut Vec<sanctifier_core::SizeWarning>,
    unsafe_patterns: &mut Vec<sanctifier_core::UnsafePattern>,
    auth_gaps: &mut Vec<String>,
    panic_issues: &mut Vec<sanctifier_core::PanicIssue>,
    arithmetic_issues: &mut Vec<sanctifier_core::ArithmeticIssue>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(
                &path,
                analyzer,
                collisions,
                size_warnings,
                unsafe_patterns,
                auth_gaps,
                panic_issues,
                arithmetic_issues,
            )?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                let file_name = path.display().to_string();

                let mut c = analyzer.scan_storage_collisions(&content);
                for i in &mut c {
                    i.location = format!("{}:{}", file_name, i.location);
                }
                collisions.extend(c);

                let s = analyzer.analyze_ledger_size(&content);
                size_warnings.extend(s);

                let mut u = analyzer.analyze_unsafe_patterns(&content);
                for i in &mut u {
                    i.snippet = format!("{}:{}", file_name, i.snippet);
                }
                unsafe_patterns.extend(u);

                for g in analyzer.scan_auth_gaps(&content) {
                    auth_gaps.push(format!("{}:{}", file_name, g));
                }

                let mut p = analyzer.scan_panics(&content);
                for i in &mut p {
                    i.location = format!("{}:{}", file_name, i.location);
                    panic_issues.push(i.clone());
                }

                let mut a = analyzer.scan_arithmetic_overflow(&content);
                for i in &mut a {
                    i.location = format!("{}:{}", file_name, i.location);
                    arithmetic_issues.push(i.clone());
                }
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
