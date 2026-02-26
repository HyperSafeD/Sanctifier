use crate::commands::webhook::{
    send_scan_completed_webhooks, ScanWebhookPayload, ScanWebhookSummary,
};
use clap::Args;
use colored::*;
use sanctifier_core::finding_codes;
use sanctifier_core::{Analyzer, SanctifyConfig, SizeWarningLevel};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::vulndb::{VulnDatabase, VulnMatch};

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

    /// Path to a custom vulnerability database JSON file
    #[arg(long)]
    pub vuln_db: Option<PathBuf>,
    /// Webhook endpoint(s) to notify when scan completes (Discord/Slack/Teams/custom)
    #[arg(long = "webhook-url")]
    pub webhook_urls: Vec<String>,
}

pub fn exec(args: AnalyzeArgs) -> anyhow::Result<()> {
    let path = &args.path;
    let format = &args.format;
    let _limit = args.limit;
    let is_json = format == "json";

    if !is_soroban_project(path) {
        if is_json {
            let err = serde_json::json!({
                "error": format!("{:?} is not a valid Soroban project", path),
                "success": false,
            });
            println!("{}", serde_json::to_string_pretty(&err)?);
        } else {
            eprintln!(
                "{} Error: {:?} is not a valid Soroban project. (Missing Cargo.toml with 'soroban-sdk' dependency)",
                "❌".red(),
                path
            );
        }
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

    let config = load_config(path);
    let analyzer = Analyzer::new(config);

    // Load vulnerability database
    let vuln_db = match &args.vuln_db {
        Some(db_path) => {
            if !is_json {
                println!(
                    "{} Loading custom vulnerability database from {:?}",
                    "📦".blue(),
                    db_path
                );
            }
            VulnDatabase::load(db_path)?
        }
        None => {
            if !is_json {
                println!(
                    "{} Loading built-in vulnerability database (v{})",
                    "📦".blue(),
                    VulnDatabase::load_default().version
                );
            }
            VulnDatabase::load_default()
        }
    };

    let mut collisions = Vec::new();
    let mut size_warnings = Vec::new();
    let mut unsafe_patterns = Vec::new();
    let mut auth_gaps = Vec::new();
    let mut panic_issues = Vec::new();
    let mut arithmetic_issues = Vec::new();
    let mut custom_matches = Vec::new();
    let mut vuln_matches: Vec<VulnMatch> = Vec::new();

    if path.is_dir() {
        walk_dir(
            path,
            &analyzer,
            &vuln_db,
            &mut collisions,
            &mut size_warnings,
            &mut unsafe_patterns,
            &mut auth_gaps,
            &mut panic_issues,
            &mut arithmetic_issues,
            &mut custom_matches,
            &mut vuln_matches,
        )?;
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        if let Ok(content) = fs::read_to_string(path) {
            let file_name = path.display().to_string();
            collisions.extend(analyzer.scan_storage_collisions(&content));
            size_warnings.extend(analyzer.analyze_ledger_size(&content));
            unsafe_patterns.extend(analyzer.analyze_unsafe_patterns(&content));
            auth_gaps.extend(analyzer.scan_auth_gaps(&content));
            panic_issues.extend(analyzer.scan_panics(&content));
            arithmetic_issues.extend(analyzer.scan_arithmetic_overflow(&content));
            custom_matches.extend(analyzer.analyze_custom_rules(&content, &analyzer.config.custom_rules));
            vuln_matches.extend(vuln_db.scan(&content, &file_name));
            custom_matches
                .extend(analyzer.analyze_custom_rules(&content, &analyzer.config.custom_rules));
        }
    }

    let total_findings = collisions.len()
        + size_warnings.len()
        + unsafe_patterns.len()
        + auth_gaps.len()
        + panic_issues.len()
        + arithmetic_issues.len()
        + custom_matches.len();

    let has_critical =
        !auth_gaps.is_empty() || panic_issues.iter().any(|p| p.issue_type == "panic!");
    let has_high = !arithmetic_issues.is_empty()
        || !panic_issues.is_empty()
        || size_warnings
            .iter()
            .any(|w| w.level == SizeWarningLevel::ExceedsLimit);
    let timestamp = chrono_timestamp();

    let webhook_payload = ScanWebhookPayload {
        event: "scan.completed",
        project_path: path.display().to_string(),
        timestamp_unix: timestamp.clone(),
        summary: ScanWebhookSummary {
            total_findings,
            has_critical,
            has_high,
        },
    };

    if let Err(err) = send_scan_completed_webhooks(&args.webhook_urls, &webhook_payload) {
        eprintln!("⚠️ Failed to initialize webhook client: {}", err);
    }

    if is_json {
        let report = serde_json::json!({
            "storage_collisions": collisions,
            "ledger_size_warnings": size_warnings,
            "unsafe_patterns": unsafe_patterns,
            "auth_gaps": auth_gaps,
            "panic_issues": panic_issues,
            "arithmetic_issues": arithmetic_issues,
            "custom_rules": custom_matches,
            "vulnerability_db_matches": vuln_matches,
            "vulnerability_db_version": vuln_db.version,
            "metadata": {
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": timestamp,
                "project_path": path.display().to_string(),
                "format": "sanctifier-ci-v1",
            },
            "error_codes": finding_codes::all_finding_codes(),
            "summary": {
                "total_findings": total_findings,
                "storage_collisions": collisions.len(),
                "auth_gaps": auth_gaps.len(),
                "panic_issues": panic_issues.len(),
                "arithmetic_issues": arithmetic_issues.len(),
                "size_warnings": size_warnings.len(),
                "unsafe_patterns": unsafe_patterns.len(),
                "custom_rule_matches": custom_matches.len(),
                "has_critical": has_critical,
                "has_high": has_high,
            },
            "findings": {
                "storage_collisions": collisions.iter().map(|c| serde_json::json!({
                    "code": finding_codes::STORAGE_COLLISION,
                    "key_value": c.key_value,
                    "key_type": c.key_type,
                    "location": c.location,
                    "message": c.message,
                })).collect::<Vec<_>>(),
                "ledger_size_warnings": size_warnings.iter().map(|w| serde_json::json!({
                    "code": finding_codes::LEDGER_SIZE_RISK,
                    "struct_name": w.struct_name,
                    "estimated_size": w.estimated_size,
                    "limit": w.limit,
                    "level": w.level,
                })).collect::<Vec<_>>(),
                "unsafe_patterns": unsafe_patterns.iter().map(|p| serde_json::json!({
                    "code": finding_codes::UNSAFE_PATTERN,
                    "pattern_type": p.pattern_type,
                    "line": p.line,
                    "snippet": p.snippet,
                })).collect::<Vec<_>>(),
                "auth_gaps": auth_gaps.iter().map(|g| serde_json::json!({
                    "code": finding_codes::AUTH_GAP,
                    "function": g,
                })).collect::<Vec<_>>(),
                "panic_issues": panic_issues.iter().map(|p| serde_json::json!({
                    "code": finding_codes::PANIC_USAGE,
                    "function_name": p.function_name,
                    "issue_type": p.issue_type,
                    "location": p.location,
                })).collect::<Vec<_>>(),
                "arithmetic_issues": arithmetic_issues.iter().map(|a| serde_json::json!({
                    "code": finding_codes::ARITHMETIC_OVERFLOW,
                    "function_name": a.function_name,
                    "operation": a.operation,
                    "suggestion": a.suggestion,
                    "location": a.location,
                })).collect::<Vec<_>>(),
                "custom_rules": custom_matches.iter().map(|m| serde_json::json!({
                    "code": finding_codes::CUSTOM_RULE_MATCH,
                    "rule_name": m.rule_name,
                    "line": m.line,
                    "snippet": m.snippet,
                    "severity": m.severity,
                })).collect::<Vec<_>>(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&report)?);

        if has_critical || has_high {
            std::process::exit(1);
        }
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
            println!(
                "   {} [{}] Value: {}",
                "->".red(),
                finding_codes::STORAGE_COLLISION.bold(),
                collision.key_value.bold()
            );
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
            println!(
                "   {} [{}] Function: {}",
                "->".red(),
                finding_codes::AUTH_GAP.bold(),
                gap.bold()
            );
        }
    }

    if panic_issues.is_empty() {
        println!("{} No explicit Panics/Unwraps found.", "✅".green());
    } else {
        println!("\n{} Found explicit Panics/Unwraps!", "⚠️".yellow());
        for issue in panic_issues {
            println!(
                "   {} [{}] Type: {}",
                "->".red(),
                finding_codes::PANIC_USAGE.bold(),
                issue.issue_type.bold()
            );
            println!("      Location: {}", issue.location);
        }
    }

    if arithmetic_issues.is_empty() {
        println!("{} No unchecked Arithmetic Operations found.", "✅".green());
    } else {
        println!("\n{} Found unchecked Arithmetic Operations!", "⚠️".yellow());
        for issue in arithmetic_issues {
            println!(
                "   {} [{}] Op: {}",
                "->".red(),
                finding_codes::ARITHMETIC_OVERFLOW.bold(),
                issue.operation.bold()
            );
            println!("      Location: {}", issue.location);
        }
    }

    if size_warnings.is_empty() {
        println!("{} No ledger size issues found.", "✅".green());
    } else {
        println!("\n{} Found Ledger Size Warnings!", "⚠️".yellow());
        for warning in size_warnings {
            println!(
                "   {} [{}] Struct: {}",
                "->".red(),
                finding_codes::LEDGER_SIZE_RISK.bold(),
                warning.struct_name.bold()
            );
            println!("      Size: {} bytes", warning.estimated_size);
        }
    }

    if !custom_matches.is_empty() {
        println!("\n{} Found Custom Rule matches!", "⚠️".yellow());
        for m in custom_matches {
            let sev_icon = match m.severity {
                sanctifier_core::RuleSeverity::Error => "❌".red(),
                sanctifier_core::RuleSeverity::Warning => "⚠️".yellow(),
                sanctifier_core::RuleSeverity::Info => "ℹ️".blue(),
            };
            println!(
                "   {} [{}|{}]: {}",
                sev_icon,
                finding_codes::CUSTOM_RULE_MATCH.bold(),
                m.rule_name.bold(),
                m.snippet
            );
        }
    }

    // Vulnerability database matches
    if vuln_matches.is_empty() {
        println!(
            "{} No known vulnerability patterns matched (DB v{}).",
            "✅".green(),
            vuln_db.version
        );
    } else {
        println!(
            "\n{} Found {} known vulnerability pattern(s) (DB v{})!",
            "🛡️".red(),
            vuln_matches.len(),
            vuln_db.version
        );
        for m in &vuln_matches {
            let sev_icon = match m.severity.as_str() {
                "critical" => "❌".red(),
                "high" => "🔴".red(),
                "medium" => "⚠️".yellow(),
                _ => "ℹ️".blue(),
            };
            println!(
                "   {} [{}] {} ({})",
                sev_icon,
                m.vuln_id.bold(),
                m.name.bold(),
                m.severity.to_uppercase()
            );
            println!("      File: {}:{}", m.file, m.line);
            println!("      {}", m.description);
            println!("      Suggestion: {}", m.recommendation);
        }
    }

    println!("\n{} Static analysis complete.", "✨".green());

    Ok(())
}

fn chrono_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    format!("{}", secs)
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
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    SanctifyConfig::default()
}

#[allow(clippy::too_many_arguments)]
fn walk_dir(
    dir: &Path,
    analyzer: &Analyzer,
    vuln_db: &VulnDatabase,
    collisions: &mut Vec<sanctifier_core::StorageCollisionIssue>,
    size_warnings: &mut Vec<sanctifier_core::SizeWarning>,
    unsafe_patterns: &mut Vec<sanctifier_core::UnsafePattern>,
    auth_gaps: &mut Vec<String>,
    panic_issues: &mut Vec<sanctifier_core::PanicIssue>,
    arithmetic_issues: &mut Vec<sanctifier_core::ArithmeticIssue>,
    custom_matches: &mut Vec<sanctifier_core::CustomRuleMatch>,
    vuln_matches: &mut Vec<VulnMatch>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip ignore_paths
            let is_ignored = analyzer
                .config
                .ignore_paths
                .iter()
                .any(|p| path.ends_with(p));
            if is_ignored {
                continue;
            }

            walk_dir(
                &path,
                analyzer,
                vuln_db,
                collisions,
                size_warnings,
                unsafe_patterns,
                auth_gaps,
                panic_issues,
                arithmetic_issues,
                custom_matches,
                vuln_matches,
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

                let mut custom =
                    analyzer.analyze_custom_rules(&content, &analyzer.config.custom_rules);
                for m in &mut custom {
                    m.snippet = format!("{}:{}: {}", file_name, m.line, m.snippet);
                }
                custom_matches.extend(custom);

                // Scan against vulnerability database
                vuln_matches.extend(vuln_db.scan(&content, &file_name));
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
