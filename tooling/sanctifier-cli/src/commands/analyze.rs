use clap::Args;
use colored::*;
use sanctifier_core::{
    Analyzer, ArithmeticIssue, PanicIssue, SanctifyConfig, SizeWarning, SymbolIssue, UnsafePattern,
};
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
    let is_json = format == "json";
    let is_junit = format == "junit";
    let is_machine = is_json || is_junit;

    if !is_soroban_project(path) {
        eprintln!(
            "{} Error: {:?} is not a valid Soroban project.",
            "❌".red(),
            path
        );
        std::process::exit(1);
    }

    if !is_machine {
        println!(
            "{} Sanctifier: Valid Soroban project found at {:?}",
            "✨".green(),
            path
        );
        println!("{} Analyzing contract at {:?}...", "🔍".blue(), path);
    }

    let analyzer = Analyzer::new(SanctifyConfig {
        ledger_limit: args.limit,
        ..Default::default()
    });

    let mut all_size_warnings = Vec::new();
    let mut all_unsafe_patterns = Vec::new();
    let mut all_auth_gaps = Vec::new();
    let mut all_panic_issues = Vec::new();
    let mut all_arithmetic_issues = Vec::new();
    let mut all_storage_collisions = Vec::new();
    let mut all_symbol_issues = Vec::new();

    if path.is_dir() {
        analyze_directory(
            path,
            &analyzer,
            &mut all_size_warnings,
            &mut all_unsafe_patterns,
            &mut all_auth_gaps,
            &mut all_panic_issues,
            &mut all_arithmetic_issues,
            &mut all_storage_collisions,
            &mut all_symbol_issues,
        );
    } else {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            analyze_file(
                path,
                &analyzer,
                &mut all_size_warnings,
                &mut all_unsafe_patterns,
                &mut all_auth_gaps,
                &mut all_panic_issues,
                &mut all_arithmetic_issues,
                &mut all_storage_collisions,
                &mut all_symbol_issues,
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
    } else if is_junit {
        print_junit_report(
            &all_size_warnings,
            &all_unsafe_patterns,
            &all_auth_gaps,
            &all_panic_issues,
            &all_arithmetic_issues,
            &all_storage_collisions,
            &all_symbol_issues,
        );
    } else {
        print_text_report(
            &all_size_warnings,
            &all_unsafe_patterns,
            &all_auth_gaps,
            &all_panic_issues,
            &all_arithmetic_issues,
            &all_storage_collisions,
            &all_symbol_issues,
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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
                    analyze_directory(
                        &path,
                        analyzer,
                        size_warnings,
                        unsafe_patterns,
                        auth_gaps,
                        panic_issues,
                        arithmetic_issues,
                        storage_collisions,
                        symbol_issues,
                    );
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                analyze_file(
                    &path,
                    analyzer,
                    size_warnings,
                    unsafe_patterns,
                    auth_gaps,
                    panic_issues,
                    arithmetic_issues,
                    storage_collisions,
                    symbol_issues,
                );
            }
        }
    }
}

fn print_text_report(
    size_warnings: &[SizeWarning],
    _unsafe_patterns: &[UnsafePattern],
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
        println!(
            "{} Found {} potential Authentication Gaps!",
            "⚠️".yellow(),
            auth_gaps.len()
        );
        for gap in auth_gaps {
            println!("   {} {}", "->".red(), gap);
        }
    }

    if symbol_issues.is_empty() {
        println!("{} No symbol length issues found.", "✅".green());
    } else {
        println!(
            "{} Found {} Symbol length issues (Soroban v20 limit)!",
            "⚠️".yellow(),
            symbol_issues.len()
        );
        for issue in symbol_issues {
            println!(
                "   {} {} ('{}') at {}",
                "->".red(),
                issue.issue_type.bold(),
                issue.value,
                issue.location
            );
        }
    }

    if panic_issues.is_empty() {
        println!(
            "{} No panic!/unwrap/expect found in contract impls.",
            "✅".green()
        );
    } else {
        println!(
            "{} Found {} potential Panic issues!",
            "⚠️".yellow(),
            panic_issues.len()
        );
        for issue in panic_issues {
            println!(
                "   {} {} in {}",
                "->".red(),
                issue.issue_type.bold(),
                issue.location
            );
        }
    }

    if arithmetic_issues.is_empty() {
        println!("{} No unchecked arithmetic issues found.", "✅".green());
    } else {
        println!(
            "{} Found {} potential Arithmetic Overflow issues!",
            "⚠️".yellow(),
            arithmetic_issues.len()
        );
        for issue in arithmetic_issues {
            println!(
                "   {} {} at {}",
                "->".red(),
                issue.operation.bold(),
                issue.location
            );
            println!("      Suggestion: {}", issue.suggestion.italic());
        }
    }

    if storage_collisions.is_empty() {
        println!("{} No storage key collisions found.", "✅".green());
    } else {
        println!(
            "{} Found {} potential Storage Key Collisions!",
            "⚠️".yellow(),
            storage_collisions.len()
        );
        for collision in storage_collisions {
            println!("   {} Value: {}", "->".red(), collision.key_value.bold());
            println!("      Location: {}", collision.location);
        }
    }

    if size_warnings.is_empty() {
        println!("{} No ledger size warnings.", "✅".green());
    } else {
        println!(
            "{} Found {} Ledger Size Warnings!",
            "⚠️".yellow(),
            size_warnings.len()
        );
        for warning in size_warnings {
            println!(
                "   {} {}: {} bytes (limit: {})",
                "->".red(),
                warning.struct_name,
                warning.estimated_size,
                warning.limit
            );
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn junit_testsuite(name: &str, cases: &[String]) -> String {
    let failures = cases.iter().filter(|c| c.contains("<failure")).count();
    let mut suite = format!(
        r#"  <testsuite name="{name}" tests="{tests}" failures="{failures}" errors="0" time="0">"#,
        name = name,
        tests = cases.len(),
        failures = failures,
    );
    suite.push('\n');
    for case in cases {
        suite.push_str(case);
        suite.push('\n');
    }
    suite.push_str("  </testsuite>");
    suite
}

fn print_junit_report(
    size_warnings: &[SizeWarning],
    unsafe_patterns: &[UnsafePattern],
    auth_gaps: &[String],
    panic_issues: &[PanicIssue],
    arithmetic_issues: &[ArithmeticIssue],
    storage_collisions: &[sanctifier_core::StorageCollisionIssue],
    symbol_issues: &[SymbolIssue],
) {
    let mut suites: Vec<String> = Vec::new();
    let mut total_tests = 0usize;
    let mut total_failures = 0usize;

    // auth_gaps
    {
        let cases: Vec<String> = if auth_gaps.is_empty() {
            total_tests += 1;
            vec![
                r#"    <testcase name="no_auth_gaps" classname="sanctifier.auth_gaps" time="0"/>"#
                    .to_string(),
            ]
        } else {
            total_tests += auth_gaps.len();
            total_failures += auth_gaps.len();
            auth_gaps.iter().enumerate().map(|(i, gap)| {
                format!(
                    r#"    <testcase name="auth_gap_{i}" classname="sanctifier.auth_gaps" time="0"><failure message="Authentication gap detected" type="AuthGap">{msg}</failure></testcase>"#,
                    i = i,
                    msg = xml_escape(gap),
                )
            }).collect()
        };
        suites.push(junit_testsuite("auth_gaps", &cases));
    }

    // symbol_issues
    {
        let cases: Vec<String> = if symbol_issues.is_empty() {
            total_tests += 1;
            vec![r#"    <testcase name="no_symbol_issues" classname="sanctifier.symbol_issues" time="0"/>"#.to_string()]
        } else {
            total_tests += symbol_issues.len();
            total_failures += symbol_issues.len();
            symbol_issues.iter().enumerate().map(|(i, s)| {
                format!(
                    r#"    <testcase name="symbol_issue_{i}" classname="sanctifier.symbol_issues" time="0"><failure message="{issue_type} symbol length violation" type="SymbolIssue">{loc}: {val}</failure></testcase>"#,
                    i = i,
                    issue_type = xml_escape(&s.issue_type),
                    loc = xml_escape(&s.location),
                    val = xml_escape(&s.value),
                )
            }).collect()
        };
        suites.push(junit_testsuite("symbol_issues", &cases));
    }

    // panic_issues
    {
        let cases: Vec<String> = if panic_issues.is_empty() {
            total_tests += 1;
            vec![r#"    <testcase name="no_panic_issues" classname="sanctifier.panic_issues" time="0"/>"#.to_string()]
        } else {
            total_tests += panic_issues.len();
            total_failures += panic_issues.len();
            panic_issues.iter().enumerate().map(|(i, p)| {
                format!(
                    r#"    <testcase name="panic_issue_{i}" classname="sanctifier.panic_issues" time="0"><failure message="{issue_type} detected" type="PanicIssue">{loc}</failure></testcase>"#,
                    i = i,
                    issue_type = xml_escape(&p.issue_type),
                    loc = xml_escape(&p.location),
                )
            }).collect()
        };
        suites.push(junit_testsuite("panic_issues", &cases));
    }

    // arithmetic_issues
    {
        let cases: Vec<String> = if arithmetic_issues.is_empty() {
            total_tests += 1;
            vec![r#"    <testcase name="no_arithmetic_issues" classname="sanctifier.arithmetic_issues" time="0"/>"#.to_string()]
        } else {
            total_tests += arithmetic_issues.len();
            total_failures += arithmetic_issues.len();
            arithmetic_issues.iter().enumerate().map(|(i, a)| {
                format!(
                    r#"    <testcase name="arithmetic_issue_{i}" classname="sanctifier.arithmetic_issues" time="0"><failure message="{op} overflow risk" type="ArithmeticIssue">{loc}: {suggestion}</failure></testcase>"#,
                    i = i,
                    op = xml_escape(&a.operation),
                    loc = xml_escape(&a.location),
                    suggestion = xml_escape(&a.suggestion),
                )
            }).collect()
        };
        suites.push(junit_testsuite("arithmetic_issues", &cases));
    }

    // storage_collisions
    {
        let cases: Vec<String> = if storage_collisions.is_empty() {
            total_tests += 1;
            vec![r#"    <testcase name="no_storage_collisions" classname="sanctifier.storage_collisions" time="0"/>"#.to_string()]
        } else {
            total_tests += storage_collisions.len();
            total_failures += storage_collisions.len();
            storage_collisions.iter().enumerate().map(|(i, s)| {
                format!(
                    r#"    <testcase name="storage_collision_{i}" classname="sanctifier.storage_collisions" time="0"><failure message="Storage key collision: {key}" type="StorageCollision">{loc}</failure></testcase>"#,
                    i = i,
                    key = xml_escape(&s.key_value),
                    loc = xml_escape(&s.location),
                )
            }).collect()
        };
        suites.push(junit_testsuite("storage_collisions", &cases));
    }

    // size_warnings
    {
        let cases: Vec<String> = if size_warnings.is_empty() {
            total_tests += 1;
            vec![r#"    <testcase name="no_size_warnings" classname="sanctifier.size_warnings" time="0"/>"#.to_string()]
        } else {
            total_tests += size_warnings.len();
            total_failures += size_warnings.len();
            size_warnings.iter().enumerate().map(|(i, w)| {
                format!(
                    r#"    <testcase name="size_warning_{i}" classname="sanctifier.size_warnings" time="0"><failure message="{name} exceeds ledger size limit" type="SizeWarning">{name}: {size} bytes (limit: {limit})</failure></testcase>"#,
                    i = i,
                    name = xml_escape(&w.struct_name),
                    size = w.estimated_size,
                    limit = w.limit,
                )
            }).collect()
        };
        suites.push(junit_testsuite("size_warnings", &cases));
    }

    // unsafe_patterns
    {
        let cases: Vec<String> = if unsafe_patterns.is_empty() {
            total_tests += 1;
            vec![r#"    <testcase name="no_unsafe_patterns" classname="sanctifier.unsafe_patterns" time="0"/>"#.to_string()]
        } else {
            total_tests += unsafe_patterns.len();
            total_failures += unsafe_patterns.len();
            unsafe_patterns.iter().enumerate().map(|(i, p)| {
                format!(
                    r#"    <testcase name="unsafe_pattern_{i}" classname="sanctifier.unsafe_patterns" time="0"><failure message="Unsafe pattern detected" type="UnsafePattern">{snippet}</failure></testcase>"#,
                    i = i,
                    snippet = xml_escape(&p.snippet),
                )
            }).collect()
        };
        suites.push(junit_testsuite("unsafe_patterns", &cases));
    }

    println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    println!(
        r#"<testsuites name="sanctifier-analysis" tests="{total_tests}" failures="{total_failures}" errors="0" time="0">"#,
        total_tests = total_tests,
        total_failures = total_failures,
    );
    for suite in &suites {
        println!("{}", suite);
    }
    println!("</testsuites>");
}

fn is_soroban_project(path: &Path) -> bool {
    if path.is_file() {
        return path.extension().and_then(|s| s.to_str()) == Some("rs")
            || path.ends_with("Cargo.toml");
    }
    path.join("Cargo.toml").exists()
}
