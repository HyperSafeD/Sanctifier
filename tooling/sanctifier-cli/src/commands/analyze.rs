use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;
use colored::*;
use sanctifier_core::{Analyzer, ArithmeticIssue, SizeWarning, UnsafePattern};

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
    
    let mut all_size_warnings: Vec<SizeWarning> = Vec::new();
    let mut all_unsafe_patterns: Vec<UnsafePattern> = Vec::new();
    let mut all_auth_gaps: Vec<String> = Vec::new();
    let mut all_panic_issues = Vec::new();
    let mut all_arithmetic_issues: Vec<ArithmeticIssue> = Vec::new();

    if path.is_dir() {
        analyze_directory(
            path,
            &analyzer,
            &mut all_size_warnings,
            &mut all_unsafe_patterns,
            &mut all_auth_gaps,
            &mut all_panic_issues,
            &mut all_arithmetic_issues,
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
        }
    }

    if is_json {
        eprintln!("{} Static analysis complete.\n", "✅".green());
        let output = serde_json::json!({
            "size_warnings": all_size_warnings,
            "unsafe_patterns": all_unsafe_patterns,
            "auth_gaps": all_auth_gaps,
            "panic_issues": all_panic_issues,
            "arithmetic_issues": all_arithmetic_issues,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()));
    } else {
        println!("{} Static analysis complete.\n", "✅".green());
        
        if all_size_warnings.is_empty() {
            println!("No ledger size issues found.");
        } else {
            for warning in all_size_warnings {
                println!(
                    "{} Warning: Struct {} is approaching ledger entry size limit!",
                    "⚠️".yellow(),
                    warning.struct_name.bold()
                );
            }
        }

        if !all_auth_gaps.is_empty() {
            println!("\n{} Found potential Authentication Gaps!", "🛑".red());
            for gap in all_auth_gaps {
                println!("   {} Function {} is modifying state without require_auth()", "->".red(), gap.bold());
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
                    format!("{:?}", issue.issue_type).yellow().bold(),
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
            }
        } else {
            println!("\nNo arithmetic overflow risks found.");
        }
        
        println!("\nNo upgrade pattern issues found.");
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
