use crate::commands::webhook::{
    send_scan_completed_webhooks, ScanWebhookPayload, ScanWebhookSummary,
};
use crate::vulndb::{VulnDatabase, VulnMatch};
use clap::{Args, ValueEnum};
use colored::*;
use rayon::prelude::*;
use sanctifier_core::finding_codes;
use sanctifier_core::{Analyzer, SanctifyConfig};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityLevel {
    Critical,
    High,
    Medium,
    Low,
}

impl SeverityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeverityLevel::Critical => "critical",
            SeverityLevel::High => "high",
            SeverityLevel::Medium => "medium",
            SeverityLevel::Low => "low",
        }
    }
}

impl std::str::FromStr for SeverityLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(SeverityLevel::Critical),
            "high" => Ok(SeverityLevel::High),
            "medium" => Ok(SeverityLevel::Medium),
            "low" => Ok(SeverityLevel::Low),
            _ => Err(format!("Invalid severity level: {}", s)),
        }
    }
}

/// Built-in analysis presets. Overrides --exit-code and --min-severity when set.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisProfile {
    /// Emit all rules; any finding triggers a non-zero exit
    Strict,
    /// Emit only critical and high findings; always exits 0
    Lenient,
    /// Emit all rules with full detail; always exits 0 (use for audit reports)
    Audit,
    /// Emit all rules; critical and high findings trigger a non-zero exit (recommended for CI)
    Ci,
}

impl AnalysisProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict  => "strict",
            Self::Lenient => "lenient",
            Self::Audit   => "audit",
            Self::Ci      => "ci",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Strict  => "all rules emitted; any finding is fatal",
            Self::Lenient => "critical+high only; always exits 0",
            Self::Audit   => "all rules emitted; always exits 0",
            Self::Ci      => "all rules emitted; critical+high findings are fatal",
        }
    }
}

#[derive(Args, Debug, Clone)]
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
    /// Per-file analysis timeout in seconds (0 = no timeout)
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
    /// Webhook endpoint(s) to notify when scan completes
    #[arg(long = "webhook-url")]
    pub webhook_urls: Vec<String>,
    /// Return non-zero exit code when findings meet or exceed severity threshold
    #[arg(long)]
    pub exit_code: bool,
    /// Minimum severity threshold for --exit-code (critical|high|medium|low)
    #[arg(long, value_enum, default_value_t = SeverityLevel::High)]
    pub min_severity: SeverityLevel,
    /// Disable incremental analysis cache
    #[arg(short = 'n', long)]
    pub no_cache: bool,
    /// Analysis profile preset — overrides --exit-code and --min-severity when set
    #[arg(long, value_enum)]
    pub profile: Option<AnalysisProfile>,
}

// ── Per-file result container ────────────────────────────────────────────────

/// All findings produced by analysing a single `.rs` file.
#[derive(Default, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct FileAnalysisResult {
    pub(crate) file_path: String,
    pub(crate) collisions: Vec<sanctifier_core::StorageCollisionIssue>,
    pub(crate) size_warnings: Vec<sanctifier_core::SizeWarning>,
    pub(crate) unsafe_patterns: Vec<sanctifier_core::UnsafePattern>,
    pub(crate) auth_gaps: Vec<sanctifier_core::AuthGapIssue>,
    pub(crate) panic_issues: Vec<sanctifier_core::PanicIssue>,
    pub(crate) arithmetic_issues: Vec<sanctifier_core::ArithmeticIssue>,
    pub(crate) custom_matches: Vec<sanctifier_core::CustomRuleMatch>,
    pub(crate) vuln_matches: Vec<VulnMatch>,
    pub(crate) event_issues: Vec<sanctifier_core::EventIssue>,
    pub(crate) unhandled_results: Vec<sanctifier_core::UnhandledResultIssue>,
    pub(crate) upgrade_reports: Vec<sanctifier_core::UpgradeReport>,
    pub(crate) smt_issues: Vec<sanctifier_core::smt::SmtInvariantIssue>,
    pub(crate) truncation_bounds_issues: Vec<sanctifier_core::TruncationBoundsIssue>,
    pub(crate) sep41_checked_contracts: Vec<String>,
    pub(crate) sep41_issues: Vec<sanctifier_core::Sep41Issue>,
    pub(crate) call_graph_edges: Vec<sanctifier_core::ContractCallEdge>,
    pub(crate) timed_out: bool,
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn exec(args: AnalyzeArgs) -> anyhow::Result<()> {
    let should_exit_on_findings = args.exit_code;
    let found_issues = run_analysis(args)?;
    if found_issues && should_exit_on_findings {
        std::process::exit(1);
    }
    Ok(())
}

pub fn run_analysis(args: AnalyzeArgs) -> anyhow::Result<bool> {
    let path_raw = args.path.clone();

    #[cfg(not(windows))]
    let path = {
        let s = path_raw.to_string_lossy();
        if s.contains('\\') {
            PathBuf::from(s.replace('\\', "/"))
        } else {
            path_raw
        }
    };

    #[cfg(windows)]
    let path = path_raw;

    let is_json = args.format == "json";
    let timeout_secs = args.timeout;
    let start = Instant::now();

    if !is_soroban_project(&path) {
        if is_json {
            let err = serde_json::json!({
                "error": format!("{:?} is not a valid Soroban project", path),
                "success": false,
            });
            println!("{}", serde_json::to_string_pretty(&err)?);
        } else {
            error!(
                target: "sanctifier",
                path = %path.display(),
                "Invalid Soroban project: missing Cargo.toml with a soroban-sdk dependency"
            );
        }
        anyhow::bail!("{:?} is not a valid Soroban project", path);
    }

    info!(target: "sanctifier", path = %path.display(), "Valid Soroban project found");
    info!(target: "sanctifier", path = %path.display(), "Analyzing contract");

    let mut config = load_config(&path);
    config.ledger_limit = args.limit;
    let analyzer = Arc::new(Analyzer::new(config));

    let vuln_db = Arc::new(match &args.vuln_db {
        Some(db_path) => {
            info!(target: "sanctifier", path = %db_path.display(), "Loading custom vulnerability database");
            VulnDatabase::load(db_path)?
        }
        None => {
            let database = VulnDatabase::load_default();
            info!(target: "sanctifier", version = %database.version, "Loading built-in vulnerability database");
            database
        }
    });

    let rs_files = if path.is_dir() {
        collect_rs_files(&path, &analyzer.config.ignore_paths)
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        vec![path.clone()]
    } else {
        vec![]
    };

    let total_files = rs_files.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let cached_counter = Arc::new(AtomicUsize::new(0));
    let timeout_dur = if timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(timeout_secs))
    };

    let cache = if args.no_cache {
        None
    } else {
        Some(Arc::new(Mutex::new(AnalysisCache::load(&path))))
    };

    let mut results: Vec<FileAnalysisResult> = rs_files
        .par_iter()
        .map(|file_path| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let file_name = file_path.display().to_string();
            if !is_json {
                eprintln!("[{}/{}] Analyzing {}", idx, total_files, file_name);
            }
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => return FileAnalysisResult::default(),
            };

            let hash = sha256_hex(&content);
            if let Some(ref cache_mutex) = cache {
                if let Some(cached_res) = cache_mutex.lock().unwrap().lookup(&file_name, &hash) {
                    cached_counter.fetch_add(1, Ordering::Relaxed);
                    return cached_res;
                }
            }

            debug!(target: "sanctifier", file = %file_name, "Scanning Rust source file");
            let analyzer = Arc::clone(&analyzer);
            let vuln_db = Arc::clone(&vuln_db);
            let file_name_clone = file_name.clone();
            let result = match run_with_timeout(timeout_dur, move || {
                analyze_single_file(&analyzer, &vuln_db, &content, &file_name_clone)
            }) {
                Some(res) => res,
                None => {
                    warn!(target: "sanctifier", file = %file_name, timeout_secs = timeout_secs, "Analysis timed out");
                    FileAnalysisResult {
                        file_path: file_name.clone(),
                        timed_out: true,
                        ..Default::default()
                    }
                }
            };

            if let Some(ref cache_mutex) = cache {
                cache_mutex
                    .lock()
                    .unwrap()
                    .store(file_name, hash, result.clone());
            }

            result
        })
        .collect();

    if let Some(cache_mutex) = cache {
        let final_cache = Arc::try_unwrap(cache_mutex).unwrap().into_inner().unwrap();
        final_cache.save();
    }

    results.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    let mut collisions = Vec::new();
    let mut size_warnings = Vec::new();
    let mut unsafe_patterns = Vec::new();
    let mut auth_gaps = Vec::new();
    let mut panic_issues = Vec::new();
    let mut arithmetic_issues = Vec::new();
    let mut custom_matches = Vec::new();
    let mut vuln_matches: Vec<VulnMatch> = Vec::new();
    let mut event_issues = Vec::new();
    let mut unhandled_results = Vec::new();
    let mut upgrade_reports = Vec::new();
    let mut smt_issues = Vec::new();
    let mut truncation_bounds_issues = Vec::new();
    let mut sep41_checked_contracts = Vec::new();
    let mut sep41_issues = Vec::new();
    let mut call_graph_edges = Vec::new();
    let mut timed_out_files: Vec<String> = Vec::new();

    for r in results {
        collisions.extend(r.collisions);
        size_warnings.extend(r.size_warnings);
        unsafe_patterns.extend(r.unsafe_patterns);
        auth_gaps.extend(r.auth_gaps);
        panic_issues.extend(r.panic_issues);
        arithmetic_issues.extend(r.arithmetic_issues);
        custom_matches.extend(r.custom_matches);
        vuln_matches.extend(r.vuln_matches);
        event_issues.extend(r.event_issues);
        unhandled_results.extend(r.unhandled_results);
        upgrade_reports.extend(r.upgrade_reports);
        smt_issues.extend(r.smt_issues);
        truncation_bounds_issues.extend(r.truncation_bounds_issues);
        sep41_checked_contracts.extend(r.sep41_checked_contracts);
        sep41_issues.extend(r.sep41_issues);
        call_graph_edges.extend(r.call_graph_edges);
        if r.timed_out {
            timed_out_files.push(r.file_path);
        }
    }

    // Apply profile filter: lenient suppresses medium/low-tier categories
    if matches!(args.profile, Some(AnalysisProfile::Lenient)) {
        collisions.clear();
        variable_shadowing_violations.clear();
        custom_matches.clear();
        contractimport_issues.clear();
        vuln_matches.retain(|v| matches!(v.severity.as_str(), "critical" | "high"));
    }

    let total_findings = collisions.len()
        + size_warnings.len()
        + unsafe_patterns.len()
        + auth_gaps.len()
        + panic_issues.len()
        + arithmetic_issues.len()
        + custom_matches.len()
        + event_issues.len()
        + unhandled_results.len()
        + upgrade_reports
            .iter()
            .map(|r| r.findings.len())
            .sum::<usize>()
        + smt_issues.len()
        + truncation_bounds_issues.len()
        + sep41_issues.len()
        + contractimport_issues.len()
        + variable_shadowing_violations.len()
        + timed_out_files.len();

    let has_critical = auth_gaps
        .iter()
        .any(|i| i.severity() == finding_codes::FindingSeverity::Critical)
        || panic_issues
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::Critical)
        || !smt_issues.is_empty()
        || sep41_issues
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::Critical)
        || size_warnings
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::Critical);

    let has_high = arithmetic_issues
        .iter()
        .any(|i| i.severity() == finding_codes::FindingSeverity::High)
        || panic_issues
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::High)
        || size_warnings
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::High)
        || unsafe_patterns
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::High)
        || upgrade_reports.iter().any(|r| {
            r.findings
                .iter()
                .any(|f| f.severity() == finding_codes::FindingSeverity::High)
        })
        || event_issues
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::High)
        || unhandled_results
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::High)
        || truncation_bounds_issues
            .iter()
            .any(|i| i.severity() == finding_codes::FindingSeverity::High);

    let highest_finding_severity: Option<SeverityLevel> = {
        let mut highest: Option<SeverityLevel> = None;
        let mut consider = |candidate: SeverityLevel| {
            highest = Some(match highest {
                Some(current) => current.max(candidate),
                None => candidate,
            });
        };
        if has_critical {
            consider(SeverityLevel::Critical);
        }
        if has_high {
            consider(SeverityLevel::High);
        }
        if !size_warnings.is_empty() || !unsafe_patterns.is_empty() || !sep41_issues.is_empty() {
            consider(SeverityLevel::Medium);
        }
        if !event_issues.is_empty() {
            consider(SeverityLevel::Low);
        }
        for vuln in &vuln_matches {
            if let Ok(sev) = vuln.severity.parse::<SeverityLevel>() {
                consider(sev);
            }
        }
        if !timed_out_files.is_empty() {
            consider(SeverityLevel::Low);
        }
        if !contractimport_issues.is_empty() {
            consider(SeverityLevel::Medium);
        }
        highest
    };

    // Profile overrides --exit-code / --min-severity when set
    let should_exit_with_1 = match args.profile {
        Some(AnalysisProfile::Strict)  => total_findings > 0,
        Some(AnalysisProfile::Lenient) => false,
        Some(AnalysisProfile::Audit)   => false,
        Some(AnalysisProfile::Ci)      => has_critical || has_high,
        None => args.exit_code && highest_finding_severity
            .map(|h| h >= args.min_severity)
            .unwrap_or(false),
    };

    let timestamp = chrono_timestamp();
    let _duration_ms = start.elapsed().as_millis() as u64;
    let duration_ms = _duration_ms;

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
        warn!(target: "sanctifier", error = %err, "Failed to initialize webhook client");
    }

    if is_json {
        let report = serde_json::json!({
            "schema_version": "1.0.0",
            "sanctifier_version": env!("CARGO_PKG_VERSION"),
            "storage_collisions": collisions,
            "call_graph": call_graph_edges,
            "ledger_size_warnings": size_warnings,
            "unsafe_patterns": unsafe_patterns,
            "auth_gaps": auth_gaps.iter().map(|g| &g.function_name).collect::<Vec<_>>(),
            "panic_issues": panic_issues,
            "arithmetic_issues": arithmetic_issues,
            "truncation_bounds_issues": truncation_bounds_issues,
            "custom_rules": custom_matches,
            "event_issues": event_issues,
            "unhandled_results": unhandled_results,
            "upgrade_reports": upgrade_reports,
            "smt_issues": smt_issues,
            "sep41_checked_contracts": sep41_checked_contracts,
            "sep41_issues": sep41_issues,
            "contractimport_issues": contractimport_issues,
            "vulnerability_db_matches": vuln_matches,
            "vulnerability_db_version": vuln_db.version,
            "timed_out_files": timed_out_files,
            "metadata": {
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": timestamp,
                "duration_ms": duration_ms,
                "project_path": path.display().to_string(),
                "format": "sanctifier-ci-v1",
                "timeout_secs": timeout_secs,
                "cached_files": cached_counter.load(Ordering::Relaxed),
                "total_files": total_files,
                "profile": args.profile.map(|p| p.as_str()),
            },
            "error_codes": finding_codes::all_finding_codes(),
            "summary": {
                "total_findings": total_findings,
                "cached_files": cached_counter.load(Ordering::Relaxed),
                "reanalysed_files": total_files - cached_counter.load(Ordering::Relaxed),
                "storage_collisions": collisions.len(),
                "auth_gaps": auth_gaps.len(),
                "panic_issues": panic_issues.len(),
                "arithmetic_issues": arithmetic_issues.len(),
                "truncation_bounds_issues": truncation_bounds_issues.len(),
                "size_warnings": size_warnings.len(),
                "unsafe_patterns": unsafe_patterns.len(),
                "custom_rule_matches": custom_matches.len(),
                "event_issues": event_issues.len(),
                "unhandled_results": unhandled_results.len(),
                "smt_issues": smt_issues.len(),
                "sep41_issues": sep41_issues.len(),
                "contractimport_issues": contractimport_issues.len(),
                "timed_out_files": timed_out_files.len(),
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
                    "function": g.function_name,
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
                "event_issues": event_issues.iter().map(|e| serde_json::json!({
                    "code": finding_codes::EVENT_INCONSISTENCY,
                    "event_name": e.event_name,
                    "issue_type": e.issue_type,
                    "location": e.location,
                    "message": e.message,
                })).collect::<Vec<_>>(),
                "unhandled_results": unhandled_results.iter().map(|r| serde_json::json!({
                    "code": finding_codes::UNHANDLED_RESULT,
                    "function_name": r.function_name,
                    "call_expression": r.call_expression,
                    "location": r.location,
                    "message": r.message,
                })).collect::<Vec<_>>(),
                "upgrade_risks": upgrade_reports.iter().flat_map(|r| &r.findings).map(|f| serde_json::json!({
                    "code": finding_codes::UPGRADE_RISK,
                    "category": f.category,
                    "function_name": f.function_name,
                    "location": f.location,
                    "message": f.message,
                    "suggestion": f.suggestion,
                })).collect::<Vec<_>>(),
                "smt_issues": smt_issues.iter().map(|s| serde_json::json!({
                    "code": finding_codes::SMT_INVARIANT_VIOLATION,
                    "function_name": s.function_name,
                    "description": s.description,
                    "location": s.location,
                })).collect::<Vec<_>>(),
                "sep41_issues": sep41_issues.iter().map(|issue| serde_json::json!({
                    "code": finding_codes::SEP41_INTERFACE_DEVIATION,
                    "function_name": issue.function_name,
                    "kind": issue.kind,
                    "location": issue.location,
                    "message": issue.message,
                    "expected_signature": issue.expected_signature,
                    "actual_signature": issue.actual_signature,
                })).collect::<Vec<_>>(),
                "timeouts": timed_out_files.iter().map(|f| serde_json::json!({
                    "code": finding_codes::ANALYSIS_TIMEOUT,
                    "file": f,
                    "message": format!("Analysis timed out after {}s", timeout_secs),
                })).collect::<Vec<_>>(),
            },
        });

        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(should_exit_with_1);
    }

    // ── Text output ──────────────────────────────────────────────────────────
    if let Some(profile) = args.profile {
        println!(
            "{} Profile: {} — {}",
            "ℹ".blue(),
            profile.as_str().bold(),
            profile.description()
        );
    }
    if !timed_out_files.is_empty() {
        println!(
            "\n{} {} file(s) timed out ({}s limit):",
            "⏱️".yellow(),
            timed_out_files.len(),
            timeout_secs
        );
        for f in &timed_out_files {
            println!(
                "   {} [{}] {}",
                "->".red(),
                finding_codes::ANALYSIS_TIMEOUT.bold(),
                f
            );
        }
    }
    if collisions.is_empty() {
        println!("\n{} No storage key collisions found.", "✅".green());
    } else {
        println!(
            "\n{} Found potential Storage Key Collisions!",
            "⚠️".yellow()
        );
        for c in &collisions {
            println!(
                "   {} [{}] Value: {}",
                "->".red(),
                finding_codes::STORAGE_COLLISION.bold(),
                c.key_value.bold()
            );
            println!("      Type: {}", c.key_type);
            println!("      Location: {}", c.location);
            println!("      Message: {}", c.message);
        }
    }
    if auth_gaps.is_empty() {
        println!("{} No authentication gaps found.", "✅".green());
    } else {
        println!("\n{} Found potential Authentication Gaps!", "⚠️".yellow());
        for gap in &auth_gaps {
            println!(
                "   {} [{}] Function: {}",
                "->".red(),
                finding_codes::AUTH_GAP.bold(),
                gap.function_name.bold()
            );
        }
    }
    if panic_issues.is_empty() {
        println!("{} No explicit Panics/Unwraps found.", "✅".green());
    } else {
        println!("\n{} Found explicit Panics/Unwraps!", "⚠️".yellow());
        for issue in &panic_issues {
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
        for issue in &arithmetic_issues {
            println!(
                "   {} [{}] Op: {}",
                "->".red(),
                finding_codes::ARITHMETIC_OVERFLOW.bold(),
                issue.operation.bold()
            );
            println!("      Location: {}", issue.location);
        }
    }
    if truncation_bounds_issues.is_empty() {
        println!(
            "{} No integer truncation or unchecked indexing found.",
            "✅".green()
        );
    } else {
        println!("\n{} Found Truncation / Bounds Risk issues!", "⚠️".yellow());
        for issue in &truncation_bounds_issues {
            println!(
                "   {} [{}] Kind: {} | Expr: {}",
                "->".red(),
                finding_codes::TRUNCATION_BOUNDS.bold(),
                issue.kind.bold(),
                issue.expression.bold()
            );
            println!("      Location: {}", issue.location);
            println!("      Suggestion: {}", issue.suggestion);
        }
    }
    if size_warnings.is_empty() {
        println!("{} No ledger size issues found.", "✅".green());
    } else {
        println!("\n{} Found Ledger Size Warnings!", "⚠️".yellow());
        for w in &size_warnings {
            println!(
                "   {} [{}] Struct: {}",
                "->".red(),
                finding_codes::LEDGER_SIZE_RISK.bold(),
                w.struct_name.bold()
            );
            println!("      Size: {} bytes", w.estimated_size);
        }
    }
    if !event_issues.is_empty() {
        println!(
            "\n{} Found Event Consistency/Optimization issues!",
            "⚠️".yellow()
        );
        for issue in &event_issues {
            println!(
                "   {} [{}] Event: {}",
                "->".red(),
                finding_codes::EVENT_INCONSISTENCY.bold(),
                issue.event_name.bold()
            );
            println!("      Type: {:?}", issue.issue_type);
            println!("      Location: {}", issue.location);
            println!("      Message: {}", issue.message);
        }
    }
    if !unhandled_results.is_empty() {
        println!("\n{} Found Unhandled Result issues!", "⚠️".yellow());
        for issue in &unhandled_results {
            println!(
                "   {} [{}] Function: {}",
                "->".red(),
                finding_codes::UNHANDLED_RESULT.bold(),
                issue.function_name.bold()
            );
            println!("      Call: {}", issue.call_expression);
            println!("      Location: {}", issue.location);
            println!("      Message: {}", issue.message);
        }
    }
    if variable_shadowing_violations.is_empty() {
        println!("{} No variable shadowing detected.", "✅".green());
    } else {
        println!("\n{} Found Variable Shadowing issues!", "⚠️".yellow());
        for violation in &variable_shadowing_violations {
            println!("   {} [S006] {}", "->".red(), violation.message.bold());
            println!("      Location: {}", violation.location);
            if let Some(suggestion) = &violation.suggestion {
                println!("      Suggestion: {}", suggestion);
            }
        }
    }
    let total_upgrade_findings: usize = upgrade_reports.iter().map(|r| r.findings.len()).sum();
    if total_upgrade_findings > 0 {
        println!("\n{} Found Upgrade/Admin Risk issues!", "⚠️".yellow());
        for report in &upgrade_reports {
            for finding in &report.findings {
                println!(
                    "   {} [{}] Category: {:?}",
                    "->".red(),
                    finding_codes::UPGRADE_RISK.bold(),
                    finding.category
                );
                if let Some(f_name) = &finding.function_name {
                    println!("      Function: {}", f_name);
                }
                println!("      Location: {}", finding.location);
                println!("      Message: {}", finding.message);
                println!("      Suggestion: {}", finding.suggestion);
            }
        }
    }
    if !smt_issues.is_empty() {
        println!("\n{} Found Formal Verification (SMT) issues!", "❌".red());
        for issue in &smt_issues {
            println!(
                "   {} [{}] Function: {}",
                "->".red(),
                finding_codes::SMT_INVARIANT_VIOLATION.bold(),
                issue.function_name.bold()
            );
            println!("      Description: {}", issue.description);
            println!("      Location: {}", issue.location);
        }
    }
    if !sep41_checked_contracts.is_empty() && sep41_issues.is_empty() {
        println!("{} SEP-41 token interface verified exactly.", "✅".green());
    } else if !sep41_issues.is_empty() {
        println!("\n{} Found SEP-41 Interface Deviations!", "⚠️".yellow());
        for issue in &sep41_issues {
            println!(
                "   {} [{}] Function: {}",
                "->".red(),
                finding_codes::SEP41_INTERFACE_DEVIATION.bold(),
                issue.function_name.bold()
            );
            println!("      Kind: {:?}", issue.kind);
            println!("      Location: {}", issue.location);
            println!("      Message: {}", issue.message);
            println!("      Expected: {}", issue.expected_signature);
            if let Some(actual) = &issue.actual_signature {
                println!("      Actual: {}", actual);
            }
        }
    }
    if !contractimport_issues.is_empty() {
        println!("\n{} Found ContractImport Mismatches!", "⚠️".yellow());
        for issue in &contractimport_issues {
            println!(
                "   {} [{}] WASM: {}",
                "->".red(),
                finding_codes::CONTRACTIMPORT_MISMATCH.bold(),
                issue.wasm_path.bold()
            );
            println!("      Location: {}", issue.location);
            println!("      Message: {}", issue.message);
        }
    }
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
            if !m.recommendation.is_empty() {
                println!("      Suggestion: {}", m.recommendation);
            }
        }
    }

    let cached_count = cached_counter.load(Ordering::Relaxed);
    let reanalysed_count = total_files - cached_count;
    println!(
        "\n{} Static analysis complete. ({} served from cache, {} re-analysed, {} ms)",
        "✨".green(),
        cached_count.to_string().bold(),
        reanalysed_count.to_string().bold(),
        duration_ms
    );
    Ok(should_exit_with_1)
}

// ── Analyse one file ─────────────────────────────────────────────────────────

pub(crate) fn analyze_single_file(
    analyzer: &Analyzer,
    vuln_db: &VulnDatabase,
    content: &str,
    file_name: &str,
) -> FileAnalysisResult {
    let mut res = FileAnalysisResult {
        file_path: file_name.to_string(),
        ..Default::default()
    };

    let mut c = analyzer.scan_storage_collisions(content);
    for i in &mut c {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.collisions = c;

    res.size_warnings = analyzer.analyze_ledger_size(content);

    let mut u = analyzer.analyze_unsafe_patterns(content);
    for i in &mut u {
        i.snippet = format!("{}:{}", file_name, i.snippet);
    }
    res.unsafe_patterns = u;

    for g in analyzer.scan_auth_gaps(content) {
        res.auth_gaps.push(sanctifier_core::AuthGapIssue {
            function_name: format!("{}:{}", file_name, g.function_name),
        });
    }

    let mut p = analyzer.scan_panics(content);
    for i in &mut p {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.panic_issues = p;

    let mut a = analyzer.scan_arithmetic_overflow(content);
    for i in &mut a {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.arithmetic_issues = a;

    let mut tb = analyzer.scan_truncation_bounds(content);
    for i in &mut tb {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.truncation_bounds_issues = tb;

    let mut custom = analyzer.analyze_custom_rules(content, &analyzer.config.custom_rules);
    for m in &mut custom {
        m.snippet = format!("{}:{}: {}", file_name, m.line, m.snippet);
    }
    res.custom_matches = custom;

    res.vuln_matches = vuln_db.scan(content, file_name);

    let mut e = analyzer.scan_events(content);
    for i in &mut e {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.event_issues = e;

    let mut r = analyzer.scan_unhandled_results(content);
    for i in &mut r {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.unhandled_results = r;

    // Scan for variable shadowing using the rule system
    let mut vs = analyzer.run_rule(content, "variable_shadowing");
    for v in &mut vs {
        v.location = format!("{}:{}", file_name, v.location);
    }
    res.variable_shadowing_violations = vs;

    let mut up = analyzer.analyze_upgrade_patterns(content);
    for f in &mut up.findings {
        f.location = format!("{}:{}", file_name, f.location);
    }
    res.upgrade_reports.push(up);

    let mut smt = analyzer.verify_smt_invariants(content);
    for i in &mut smt {
        i.location = format!("{}:{}", file_name, i.location);
    }
    res.smt_issues = smt;

    let sep41_report = analyzer.verify_sep41_interface(content);
    if sep41_report.candidate {
        res.sep41_checked_contracts.push(file_name.to_string());
        for mut issue in sep41_report.issues {
            issue.location = format!("{}:{}", file_name, issue.location);
            res.sep41_issues.push(issue);
        }
    }

    // Call graph edges
    let inferred_caller = infer_contract_name(content).unwrap_or_else(|| {
        Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_string()
    });
    let mut edges = analyzer.scan_invoke_contract_calls(content, &inferred_caller, file_name);
    for edge in &mut edges {
        if let Some(ref mut func) = edge.function_expr {
            if func.starts_with('&') {
                *func = func.trim_start_matches('&').trim().to_string();
            }
        }
    }
    res.call_graph_edges = edges;

    res
}

// ── Timeout wrapper ──────────────────────────────────────────────────────────

pub(crate) fn run_with_timeout<F, R>(timeout: Option<Duration>, f: F) -> Option<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    match timeout {
        None => Some(f()),
        Some(dur) => {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(f());
            });
            rx.recv_timeout(dur).ok()
        }
    }
}

// ── File collection ──────────────────────────────────────────────────────────

pub(crate) fn collect_rs_files(dir: &Path, ignore_paths: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_rs_files_inner(dir, ignore_paths, &mut out);
    out
}

fn collect_rs_files_inner(dir: &Path, ignore_paths: &[String], out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !ignore_paths.iter().any(|p| path.ends_with(p)) {
                collect_rs_files_inner(&path, ignore_paths, out);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn chrono_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

pub(crate) fn load_config(path: &Path) -> SanctifyConfig {
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

pub(crate) fn is_soroban_project(path: &Path) -> bool {
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

pub(crate) fn infer_contract_name(source: &str) -> Option<String> {
    let mut saw_contract_attr = false;
    for line in source.lines() {
        let l = line.trim();
        if l.starts_with("#[contract]") {
            saw_contract_attr = true;
            continue;
        }
        if saw_contract_attr {
            if let Some(rest) = l.strip_prefix("pub struct ") {
                return Some(
                    rest.trim_end_matches('{')
                        .trim_end_matches(';')
                        .split_whitespace()
                        .next()?
                        .to_string(),
                );
            }
            if let Some(rest) = l.strip_prefix("struct ") {
                return Some(
                    rest.trim_end_matches('{')
                        .trim_end_matches(';')
                        .split_whitespace()
                        .next()?
                        .to_string(),
                );
            }
        }
    }
    None
}
