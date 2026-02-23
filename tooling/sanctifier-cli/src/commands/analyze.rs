use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;
use colored::*;
use sanctifier_core::{Analyzer, SanctifyConfig, SizeWarning, UnsafePattern, ArithmeticIssue, PanicIssue, SymbolIssue};

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
    let is_json = format == "json";

    if !is_soroban_project(path) {
        eprintln!(
            "{} Error: {:?} is not a valid Soroban project.",
            "❌".red(),
            path
        );
        std::process::exit(1);
    }

    if !is_json {
        println!("{} Sanctifier: Valid Soroban project found at {:?}", "✨".green(), path);
        println!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
    }

    let mut config = SanctifyConfig::default();
    config.ledger_limit = args.limit;
    let analyzer = Analyzer::new(config);
    
    let mut all_size_warnings = Vec::new();
    let mut all_unsafe_patterns = Vec::new();
    let mut all_auth_gaps = Vec::new();
    let mut all_panic_issues = Vec::new();
    let mut all_arithmetic_issues = Vec::new();
    let mut all_storage_collisions = Vec::new();
    let mut all_symbol_issues = Vec::new();

    if path.is_dir() {
        analyze_directory(
            path, &analyzer, &mut all_size_warnings, &mut all_unsafe_patterns, &mut all_auth_gaps,
            &mut all_panic_issues, &mut all_arithmetic_issues, &mut all_storage_collisions, &mut all_symbol_issues
        );
    } else {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            analyze_file(
                path, &analyzer, &mut all_size_warnings, &mut all_unsafe_patterns, &mut all_auth_gaps,
                &mut all_panic_issues, &mut all_arithmetic_issues, &mut all_storage_collisions, &mut all_symbol_issues
            );
        }
    }

    if is_json {
        let report = serde_json::json!({
            "size_warnings": all_size_warnings,
            "unsafe_patterns": all_unsafe_patterns,
            "auth_gaps": all_auth_gaps,
            "panic_issues": all_panic_issues,
            "arithmetic_issues": all_arithmetic_issues,
            "storage_collisions": all_storage_collisions,
            "symbol_issues": all_symbol_issues,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text_report(
            &all_size_warnings, &all_unsafe_patterns, &all_auth_gaps,
            &all_panic_issues, &all_arithmetic_issues, &all_storage_collisions, &all_symbol_issues
        );
    }
    
    Ok(())
}

fn analyze_file(
    path: &Path,
    analyzer: &Analyzer,
    size_warnings: &mut Vec<SizeWarning>,
    unsafe_patterns: &mut Vec<UnsafePattern>,
    auth_gaps: &mut Vec<String>,
    panic_issues: &mut Vec<PanicIssue>,
    arithmetic_issues: &mut Vec<ArithmeticIssue>,
    storage_collisions: &mut Vec<sanctifier_core::StorageCollisionIssue>,
    symbol_issues: &mut Vec<SymbolIssue>,
) {
    if let Ok(content) = fs::read_to_string(path) {
        let file_path = path.display().to_string();

        // Ledger size
        for mut w in analyzer.analyze_ledger_size(&content) {
            w.struct_name = format!("{} in {}", w.struct_name, file_path);
            size_warnings.push(w);
        }

        // Unsafe patterns
        for mut p in analyzer.analyze_unsafe_patterns(&content) {
            p.snippet = format!("{}: {}", file_path, p.snippet);
            unsafe_patterns.push(p);
        }

        // Auth gaps
        for g in analyzer.scan_auth_gaps(&content) {
            auth_gaps.push(format!("{}: {}", file_path, g));
        }

        // Panics
        for mut p in analyzer.scan_panics(&content) {
            p.location = format!("{}: {}", file_path, p.location);
            panic_issues.push(p);
        }

        // Arithmetic
        for mut a in analyzer.scan_arithmetic_overflow(&content) {
            a.location = format!("{}: {}", file_path, a.location);
            arithmetic_issues.push(a);
        }

        // Storage collisions
        for mut s in analyzer.scan_storage_collisions(&content) {
            s.location = format!("{}: {}", file_path, s.location);
            storage_collisions.push(s);
        }

        // Symbol issues (v20)
        for mut s in analyzer.scan_symbols(&content) {
            s.location = format!("{}: {}", file_path, s.location);
            symbol_issues.push(s);
        }
    }
}

fn analyze_directory(
    dir: &Path,
    analyzer: &Analyzer,
    size_warnings: &mut Vec<SizeWarning>,
    unsafe_patterns: &mut Vec<UnsafePattern>,
    auth_gaps: &mut Vec<String>,
    panic_issues: &mut Vec<PanicIssue>,
    arithmetic_issues: &mut Vec<ArithmeticIssue>,
    storage_collisions: &mut Vec<sanctifier_core::StorageCollisionIssue>,
    symbol_issues: &mut Vec<SymbolIssue>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !path.ends_with("target") && !path.ends_with(".git") {
                    analyze_directory(&path, analyzer, size_warnings, unsafe_patterns, auth_gaps, panic_issues, arithmetic_issues, storage_collisions, symbol_issues);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                analyze_file(&path, analyzer, size_warnings, unsafe_patterns, auth_gaps, panic_issues, arithmetic_issues, storage_collisions, symbol_issues);
            }
        }
    }
}

fn print_text_report(
    size_warnings: &[SizeWarning],
    unsafe_patterns: &[UnsafePattern],
    auth_gaps: &[String],
    panic_issues: &[PanicIssue],
    arithmetic_issues: &[ArithmeticIssue],
    storage_collisions: &[sanctifier_core::StorageCollisionIssue],
    symbol_issues: &[SymbolIssue],
) {
    println!("\n{}", "--- Analysis Results ---".bold());

    if auth_gaps.is_empty() {
        println!("{} No authentication gaps found.", "✅".green());
    } else {
        println!("{} Found {} potential Authentication Gaps!", "⚠️".yellow(), auth_gaps.len());
        for gap in auth_gaps {
            println!("   {} {}", "->".red(), gap);
        }
    }

    if symbol_issues.is_empty() {
        println!("{} No symbol length issues found.", "✅".green());
    } else {
        println!("{} Found {} Symbol length issues (Soroban v20 limit)!", "⚠️".yellow(), symbol_issues.len());
        for issue in symbol_issues {
            println!("   {} {} ('{}') at {}", "->".red(), issue.issue_type.bold(), issue.value, issue.location);
        }
    }

    if panic_issues.is_empty() {
        println!("{} No panic!/unwrap/expect found in contract impls.", "✅".green());
    } else {
        println!("{} Found {} potential Panic issues!", "⚠️".yellow(), panic_issues.len());
        for issue in panic_issues {
            println!("   {} {} in {}", "->".red(), issue.issue_type.bold(), issue.location);
        }
    }

    if arithmetic_issues.is_empty() {
        println!("{} No unchecked arithmetic issues found.", "✅".green());
    } else {
        println!("{} Found {} potential Arithmetic Overflow issues!", "⚠️".yellow(), arithmetic_issues.len());
        for issue in arithmetic_issues {
            println!("   {} {} at {}", "->".red(), issue.operation.bold(), issue.location);
            println!("      Suggestion: {}", issue.suggestion.italic());
        }
    }

    if storage_collisions.is_empty() {
        println!("{} No storage key collisions found.", "✅".green());
    } else {
        println!("{} Found {} potential Storage Key Collisions!", "⚠️".yellow(), storage_collisions.len());
        for collision in storage_collisions {
            println!("   {} Value: {}", "->".red(), collision.key_value.bold());
            println!("      Location: {}", collision.location);
        }
    }

    if size_warnings.is_empty() {
        println!("{} No ledger size warnings.", "✅".green());
    } else {
        println!("{} Found {} Ledger Size Warnings!", "⚠️".yellow(), size_warnings.len());
        for warning in size_warnings {
            println!("   {} {}: {} bytes (limit: {})", "->".red(), warning.struct_name, warning.estimated_size, warning.limit);
        }
    }
}

fn is_soroban_project(path: &Path) -> bool {
    if path.is_file() {
        return path.extension().and_then(|s| s.to_str()) == Some("rs") || path.ends_with("Cargo.toml");
    }
    path.join("Cargo.toml").exists()
}
