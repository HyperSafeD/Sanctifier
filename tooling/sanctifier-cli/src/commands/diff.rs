use crate::commands::analyze::{
    analyze_single_file, collect_rs_files, is_soroban_project, load_config, run_with_timeout,
    FileAnalysisResult, SeverityLevel,
};
use crate::vulndb::{VulnDatabase, VulnMatch};
use clap::Args;
use colored::*;
use rayon::prelude::*;
use sanctifier_core::{Analyzer, SanctifyConfig};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// Path to the contract directory or a single .rs file
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Path to the baseline JSON file (output of `sanctifier analyze --format json`)
    #[arg(long)]
    pub baseline: PathBuf,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Overwrite the baseline file with current results
    #[arg(long)]
    pub update_baseline: bool,

    /// Return non-zero exit code when new findings meet or exceed severity threshold
    #[arg(long)]
    pub exit_code: bool,

    /// Minimum severity threshold for --exit-code (critical|high|medium|low)
    #[arg(long, value_enum, default_value_t = SeverityLevel::High)]
    pub min_severity: SeverityLevel,

    /// Per-file analysis timeout in seconds (0 = no timeout)
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,

    /// Path to a custom vulnerability database JSON file
    #[arg(long)]
    pub vuln_db: Option<PathBuf>,

    /// Limit for ledger entry size in bytes
    #[arg(short, long, default_value = "64000")]
    pub limit: usize,
}

// ---------------------------------------------------------------------------
// Fingerprinting
// ---------------------------------------------------------------------------

/// Build a stable fingerprint for a finding from the JSON report.
///
/// We intentionally avoid exact line numbers so that minor code shifts don't
/// invalidate the baseline.  Instead we key on:
///   - finding category (e.g. "auth_gap", "arithmetic_overflow")
///   - file path (extracted from the `location` / `function_name` field)
///   - a distinguishing detail (function name, operation, struct name, etc.)
fn fingerprint_finding(category: &str, detail: &str) -> String {
    format!("{}::{}", category, detail)
}

fn extract_fingerprints_from_json(report: &Value) -> HashSet<String> {
    let mut fps = HashSet::new();

    // auth_gaps – function_name contains "file:fn_name"
    if let Some(arr) = report.get("auth_gaps").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(name) = item.get("function_name").and_then(|v| v.as_str()) {
                fps.insert(fingerprint_finding("auth_gap", name));
            }
        }
    }

    // arithmetic_issues – keyed on operation + location
    if let Some(arr) = report.get("arithmetic_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let op = item.get("operation").and_then(|v| v.as_str()).unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            // Use file + function from location (strip line number)
            let stable_loc = strip_line_number(loc);
            fps.insert(fingerprint_finding(
                "arithmetic_overflow",
                &format!("{}::{}", stable_loc, op),
            ));
        }
    }

    // panic_issues
    if let Some(arr) = report.get("panic_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let issue_type = item
                .get("issue_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            let stable_loc = strip_line_number(loc);
            fps.insert(fingerprint_finding(
                "panic_usage",
                &format!("{}::{}", stable_loc, issue_type),
            ));
        }
    }

    // storage_collisions
    if let Some(arr) = report.get("storage_collisions").and_then(|v| v.as_array()) {
        for item in arr {
            let key = item.get("key_value").and_then(|v| v.as_str()).unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            let stable_loc = strip_line_number(loc);
            fps.insert(fingerprint_finding(
                "storage_collision",
                &format!("{}::{}", stable_loc, key),
            ));
        }
    }

    // unsafe_patterns
    if let Some(arr) = report.get("unsafe_patterns").and_then(|v| v.as_array()) {
        for item in arr {
            let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            fps.insert(fingerprint_finding("unsafe_pattern", snippet));
        }
    }

    // ledger_size_warnings
    if let Some(arr) = report
        .get("ledger_size_warnings")
        .and_then(|v| v.as_array())
    {
        for item in arr {
            let name = item
                .get("struct_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            fps.insert(fingerprint_finding("ledger_size_risk", name));
        }
    }

    // event_issues
    if let Some(arr) = report.get("event_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let name = item
                .get("event_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            let stable_loc = strip_line_number(loc);
            fps.insert(fingerprint_finding(
                "event_issue",
                &format!("{}::{}", stable_loc, name),
            ));
        }
    }

    // unhandled_results
    if let Some(arr) = report.get("unhandled_results").and_then(|v| v.as_array()) {
        for item in arr {
            let fname = item
                .get("function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let call = item
                .get("call_expression")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            let stable_loc = strip_line_number(loc);
            fps.insert(fingerprint_finding(
                "unhandled_result",
                &format!("{}::{}::{}", stable_loc, fname, call),
            ));
        }
    }

    // upgrade_reports
    if let Some(arr) = report.get("upgrade_reports").and_then(|v| v.as_array()) {
        for rpt in arr {
            if let Some(findings) = rpt.get("findings").and_then(|v| v.as_array()) {
                for item in findings {
                    let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                    let msg = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    let stable_loc = strip_line_number(loc);
                    fps.insert(fingerprint_finding(
                        "upgrade_risk",
                        &format!("{}::{}", stable_loc, msg),
                    ));
                }
            }
        }
    }

    // smt_issues
    if let Some(arr) = report.get("smt_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let fname = item
                .get("function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let desc = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            fps.insert(fingerprint_finding(
                "smt_invariant",
                &format!("{}::{}", fname, desc),
            ));
        }
    }

    // sep41_issues
    if let Some(arr) = report.get("sep41_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let fname = item
                .get("function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let msg = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
            fps.insert(fingerprint_finding(
                "sep41_deviation",
                &format!("{}::{}", fname, msg),
            ));
        }
    }

    // vulnerability_db_matches
    if let Some(arr) = report
        .get("vulnerability_db_matches")
        .and_then(|v| v.as_array())
    {
        for item in arr {
            let vid = item.get("vuln_id").and_then(|v| v.as_str()).unwrap_or("");
            let file = item.get("file").and_then(|v| v.as_str()).unwrap_or("");
            fps.insert(fingerprint_finding(
                "vuln_db",
                &format!("{}::{}", file, vid),
            ));
        }
    }

    // custom_rules
    if let Some(arr) = report.get("custom_rules").and_then(|v| v.as_array()) {
        for item in arr {
            let name = item.get("rule_name").and_then(|v| v.as_str()).unwrap_or("");
            let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            fps.insert(fingerprint_finding(
                "custom_rule",
                &format!("{}::{}", name, snippet),
            ));
        }
    }

    fps
}

/// Strip trailing `:line_number` from a location string to make fingerprints
/// stable across minor line-number changes.
///
/// Example: `"src/lib.rs:fn_name:42"` -> `"src/lib.rs:fn_name"`
fn strip_line_number(loc: &str) -> &str {
    // If the last segment after ':' is purely numeric, strip it.
    if let Some(pos) = loc.rfind(':') {
        let tail = &loc[pos + 1..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return &loc[..pos];
        }
    }
    loc
}

// ---------------------------------------------------------------------------
// Collect new findings as JSON items for reporting
// ---------------------------------------------------------------------------

/// Given a current report and baseline fingerprints, extract only the new
/// finding entries (grouped by category).
fn collect_new_findings(current: &Value, baseline_fps: &HashSet<String>) -> Value {
    let mut new_report = serde_json::Map::new();

    fn filter_array(
        current: &Value,
        key: &str,
        fp_fn: impl Fn(&Value) -> Option<String>,
        baseline: &HashSet<String>,
    ) -> Vec<Value> {
        current
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|item| {
                        fp_fn(item)
                            .map(|fp| !baseline.contains(&fp))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    new_report.insert(
        "auth_gaps".into(),
        serde_json::json!(filter_array(
            current,
            "auth_gaps",
            |item| {
                item.get("function_name")
                    .and_then(|v| v.as_str())
                    .map(|n| fingerprint_finding("auth_gap", n))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "arithmetic_issues".into(),
        serde_json::json!(filter_array(
            current,
            "arithmetic_issues",
            |item| {
                let op = item.get("operation").and_then(|v| v.as_str()).unwrap_or("");
                let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "arithmetic_overflow",
                    &format!("{}::{}", strip_line_number(loc), op),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "panic_issues".into(),
        serde_json::json!(filter_array(
            current,
            "panic_issues",
            |item| {
                let it = item
                    .get("issue_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "panic_usage",
                    &format!("{}::{}", strip_line_number(loc), it),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "storage_collisions".into(),
        serde_json::json!(filter_array(
            current,
            "storage_collisions",
            |item| {
                let key = item.get("key_value").and_then(|v| v.as_str()).unwrap_or("");
                let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "storage_collision",
                    &format!("{}::{}", strip_line_number(loc), key),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "unsafe_patterns".into(),
        serde_json::json!(filter_array(
            current,
            "unsafe_patterns",
            |item| {
                item.get("snippet")
                    .and_then(|v| v.as_str())
                    .map(|s| fingerprint_finding("unsafe_pattern", s))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "ledger_size_warnings".into(),
        serde_json::json!(filter_array(
            current,
            "ledger_size_warnings",
            |item| {
                item.get("struct_name")
                    .and_then(|v| v.as_str())
                    .map(|n| fingerprint_finding("ledger_size_risk", n))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "event_issues".into(),
        serde_json::json!(filter_array(
            current,
            "event_issues",
            |item| {
                let name = item
                    .get("event_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "event_issue",
                    &format!("{}::{}", strip_line_number(loc), name),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "unhandled_results".into(),
        serde_json::json!(filter_array(
            current,
            "unhandled_results",
            |item| {
                let fname = item
                    .get("function_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let call = item
                    .get("call_expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "unhandled_result",
                    &format!("{}::{}::{}", strip_line_number(loc), fname, call),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert("upgrade_reports".into(), {
        let mut new_reports = Vec::new();
        if let Some(arr) = current.get("upgrade_reports").and_then(|v| v.as_array()) {
            for rpt in arr {
                if let Some(findings) = rpt.get("findings").and_then(|v| v.as_array()) {
                    let new_findings: Vec<Value> = findings
                        .iter()
                        .filter(|item| {
                            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
                            let msg = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
                            let fp = fingerprint_finding(
                                "upgrade_risk",
                                &format!("{}::{}", strip_line_number(loc), msg),
                            );
                            !baseline_fps.contains(&fp)
                        })
                        .cloned()
                        .collect();
                    if !new_findings.is_empty() {
                        let mut new_rpt = rpt.clone();
                        new_rpt
                            .as_object_mut()
                            .unwrap()
                            .insert("findings".into(), serde_json::json!(new_findings));
                        new_reports.push(new_rpt);
                    }
                }
            }
        }
        serde_json::json!(new_reports)
    });

    new_report.insert(
        "smt_issues".into(),
        serde_json::json!(filter_array(
            current,
            "smt_issues",
            |item| {
                let fname = item
                    .get("function_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let desc = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(fingerprint_finding(
                    "smt_invariant",
                    &format!("{}::{}", fname, desc),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "sep41_issues".into(),
        serde_json::json!(filter_array(
            current,
            "sep41_issues",
            |item| {
                let fname = item
                    .get("function_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let msg = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "sep41_deviation",
                    &format!("{}::{}", fname, msg),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "vulnerability_db_matches".into(),
        serde_json::json!(filter_array(
            current,
            "vulnerability_db_matches",
            |item| {
                let vid = item.get("vuln_id").and_then(|v| v.as_str()).unwrap_or("");
                let file = item.get("file").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "vuln_db",
                    &format!("{}::{}", file, vid),
                ))
            },
            baseline_fps
        )),
    );

    new_report.insert(
        "custom_rules".into(),
        serde_json::json!(filter_array(
            current,
            "custom_rules",
            |item| {
                let name = item.get("rule_name").and_then(|v| v.as_str()).unwrap_or("");
                let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
                Some(fingerprint_finding(
                    "custom_rule",
                    &format!("{}::{}", name, snippet),
                ))
            },
            baseline_fps
        )),
    );

    Value::Object(new_report)
}

fn count_new_findings(new: &Value) -> usize {
    let simple_keys = [
        "auth_gaps",
        "arithmetic_issues",
        "panic_issues",
        "storage_collisions",
        "unsafe_patterns",
        "ledger_size_warnings",
        "event_issues",
        "unhandled_results",
        "smt_issues",
        "sep41_issues",
        "vulnerability_db_matches",
        "custom_rules",
    ];
    let mut total = 0usize;
    for key in &simple_keys {
        if let Some(arr) = new.get(*key).and_then(|v| v.as_array()) {
            total += arr.len();
        }
    }
    if let Some(arr) = new.get("upgrade_reports").and_then(|v| v.as_array()) {
        for rpt in arr {
            if let Some(findings) = rpt.get("findings").and_then(|v| v.as_array()) {
                total += findings.len();
            }
        }
    }
    total
}

/// Determine the highest severity among new findings.
fn highest_severity_in_new(new: &Value) -> Option<SeverityLevel> {
    let mut highest: Option<SeverityLevel> = None;
    let mut consider = |s: SeverityLevel| {
        highest = Some(match highest {
            Some(cur) => cur.max(s),
            None => s,
        });
    };

    // auth_gaps -> Critical
    if new
        .get("auth_gaps")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::Critical);
    }
    // smt_issues -> Critical
    if new
        .get("smt_issues")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::Critical);
    }
    // arithmetic_issues -> High
    if new
        .get("arithmetic_issues")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::High);
    }
    // panic_issues -> High
    if new
        .get("panic_issues")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::High);
    }
    // unsafe_patterns -> High
    if new
        .get("unsafe_patterns")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::High);
    }
    // upgrade_reports -> High
    if new
        .get("upgrade_reports")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::High);
    }
    // unhandled_results -> High
    if new
        .get("unhandled_results")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::High);
    }
    // storage_collisions -> Medium
    if new
        .get("storage_collisions")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::Medium);
    }
    // ledger_size_warnings -> Medium
    if new
        .get("ledger_size_warnings")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::Medium);
    }
    // sep41_issues -> Medium
    if new
        .get("sep41_issues")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::Medium);
    }
    // event_issues -> Low
    if new
        .get("event_issues")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        consider(SeverityLevel::Low);
    }
    // vuln_db matches – parse per-item severity
    if let Some(arr) = new
        .get("vulnerability_db_matches")
        .and_then(|v| v.as_array())
    {
        for item in arr {
            if let Some(sev_str) = item.get("severity").and_then(|v| v.as_str()) {
                if let Ok(sev) = sev_str.parse::<SeverityLevel>() {
                    consider(sev);
                }
            }
        }
    }

    highest
}

// ---------------------------------------------------------------------------
// Build current JSON report (same format as `sanctifier analyze --format json`)
// ---------------------------------------------------------------------------

fn build_current_report(
    path: &Path,
    config: &SanctifyConfig,
    analyzer: &Arc<Analyzer>,
    vuln_db: &Arc<VulnDatabase>,
    timeout_secs: u64,
) -> anyhow::Result<Value> {
    let rs_files = if path.is_dir() {
        collect_rs_files(path, &config.ignore_paths)
    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        vec![path.to_path_buf()]
    } else {
        vec![]
    };

    let total_files = rs_files.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let timeout_dur = if timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(timeout_secs))
    };

    let mut results: Vec<FileAnalysisResult> = rs_files
        .par_iter()
        .map(|file_path| {
            let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
            let file_name = file_path.display().to_string();
            eprintln!("[{}/{}] Analyzing {}", idx, total_files, file_name);
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => return FileAnalysisResult::default(),
            };
            let analyzer = Arc::clone(analyzer);
            let vuln_db = Arc::clone(vuln_db);
            let file_name_clone = file_name.clone();
            match run_with_timeout(timeout_dur, move || {
                analyze_single_file(&analyzer, &vuln_db, &content, &file_name_clone)
            }) {
                Some(res) => res,
                None => {
                    warn!(target: "sanctifier", file = %file_name, timeout_secs = timeout_secs, "Analysis timed out");
                    FileAnalysisResult { file_path: file_name, timed_out: true, ..Default::default() }
                }
            }
        })
        .collect();

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
    let mut sep41_checked_contracts = Vec::new();
    let mut sep41_issues = Vec::new();
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
        sep41_checked_contracts.extend(r.sep41_checked_contracts);
        sep41_issues.extend(r.sep41_issues);
        if r.timed_out {
            timed_out_files.push(r.file_path);
        }
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
        + sep41_issues.len()
        + timed_out_files.len();

    let report = serde_json::json!({
        "schema_version": "1.0.0",
        "storage_collisions": collisions,
        "ledger_size_warnings": size_warnings,
        "unsafe_patterns": unsafe_patterns,
        "auth_gaps": auth_gaps,
        "panic_issues": panic_issues,
        "arithmetic_issues": arithmetic_issues,
        "custom_rules": custom_matches,
        "event_issues": event_issues,
        "unhandled_results": unhandled_results,
        "upgrade_reports": upgrade_reports,
        "smt_issues": smt_issues,
        "sep41_checked_contracts": sep41_checked_contracts,
        "sep41_issues": sep41_issues,
        "vulnerability_db_matches": vuln_matches,
        "vulnerability_db_version": vuln_db.version,
        "timed_out_files": timed_out_files,
        "metadata": {
            "version": env!("CARGO_PKG_VERSION"),
            "project_path": path.display().to_string(),
            "format": "sanctifier-ci-v1",
            "timeout_secs": timeout_secs,
        },
        "summary": {
            "total_findings": total_findings,
        },
    });

    Ok(report)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn exec(args: DiffArgs) -> anyhow::Result<()> {
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
        std::process::exit(2);
    }

    info!(target: "sanctifier", path = %path.display(), "Analyzing contract for diff");

    let mut config = load_config(&path);
    config.ledger_limit = args.limit;
    let analyzer = Arc::new(Analyzer::new(config.clone()));

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

    // 1. Build current report
    let current_report = build_current_report(&path, &config, &analyzer, &vuln_db, args.timeout)?;

    // 2. Handle --update-baseline: write current results and potentially also diff
    if args.update_baseline {
        let json_str = serde_json::to_string_pretty(&current_report)?;
        fs::write(&args.baseline, &json_str)?;
        if !is_json {
            println!(
                "{} Baseline updated: {}",
                "✅".green(),
                args.baseline.display()
            );
        }
    }

    // 3. Load baseline
    let baseline_report: Value = if args.baseline.exists() {
        let baseline_str = fs::read_to_string(&args.baseline)?;
        serde_json::from_str(&baseline_str)?
    } else {
        if !args.update_baseline {
            if is_json {
                let err = serde_json::json!({
                    "error": format!("Baseline file not found: {}", args.baseline.display()),
                    "success": false,
                });
                println!("{}", serde_json::to_string_pretty(&err)?);
            } else {
                error!(
                    target: "sanctifier",
                    path = %args.baseline.display(),
                    "Baseline file not found. Run with --update-baseline to create one."
                );
            }
            std::process::exit(2);
        }
        // If we just wrote the baseline, there are no new findings by definition
        current_report.clone()
    };

    // 4. Compare
    let baseline_fps = extract_fingerprints_from_json(&baseline_report);
    let new_findings = collect_new_findings(&current_report, &baseline_fps);
    let new_count = count_new_findings(&new_findings);

    let duration_ms = start.elapsed().as_millis() as u64;

    // 5. Determine exit code
    let new_highest = highest_severity_in_new(&new_findings);
    let should_exit_with_1 =
        args.exit_code && new_highest.map(|h| h >= args.min_severity).unwrap_or(false);

    // 6. Output
    if is_json {
        let diff_report = serde_json::json!({
            "new_findings": new_findings,
            "new_findings_count": new_count,
            "baseline_path": args.baseline.display().to_string(),
            "metadata": {
                "version": env!("CARGO_PKG_VERSION"),
                "duration_ms": duration_ms,
                "project_path": path.display().to_string(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&diff_report)?);
    } else {
        println!(
            "\n{} Diff analysis complete. ({} ms)",
            "✨".green(),
            duration_ms
        );
        println!("   Baseline: {}", args.baseline.display());

        if new_count == 0 {
            println!("   {} No new findings compared to baseline.", "✅".green());
        } else {
            println!(
                "   {} {} new finding(s) compared to baseline:",
                "⚠️".yellow(),
                new_count
            );

            print_new_text_findings(&new_findings);
        }
    }

    if should_exit_with_1 {
        std::process::exit(1);
    }
    Ok(())
}

fn print_new_text_findings(new: &Value) {
    use sanctifier_core::finding_codes;

    if let Some(arr) = new.get("auth_gaps").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(name) = item.get("function_name").and_then(|v| v.as_str()) {
                println!(
                    "   {} [{}] Auth gap: {}",
                    "->".red(),
                    finding_codes::AUTH_GAP.bold(),
                    name.bold()
                );
            }
        }
    }
    if let Some(arr) = new.get("arithmetic_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let op = item.get("operation").and_then(|v| v.as_str()).unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "   {} [{}] Arithmetic: {} at {}",
                "->".red(),
                finding_codes::ARITHMETIC_OVERFLOW.bold(),
                op.bold(),
                loc
            );
        }
    }
    if let Some(arr) = new.get("panic_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let it = item
                .get("issue_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let loc = item.get("location").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "   {} [{}] Panic: {} at {}",
                "->".red(),
                finding_codes::PANIC_USAGE.bold(),
                it.bold(),
                loc
            );
        }
    }
    if let Some(arr) = new.get("storage_collisions").and_then(|v| v.as_array()) {
        for item in arr {
            let key = item.get("key_value").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "   {} [{}] Storage collision: {}",
                "->".red(),
                finding_codes::STORAGE_COLLISION.bold(),
                key.bold()
            );
        }
    }
    if let Some(arr) = new.get("unsafe_patterns").and_then(|v| v.as_array()) {
        for item in arr {
            let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "   {} [{}] Unsafe: {}",
                "->".red(),
                finding_codes::UNSAFE_PATTERN.bold(),
                snippet
            );
        }
    }
    if let Some(arr) = new.get("ledger_size_warnings").and_then(|v| v.as_array()) {
        for item in arr {
            let name = item
                .get("struct_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!(
                "   {} [{}] Size: {}",
                "->".red(),
                finding_codes::LEDGER_SIZE_RISK.bold(),
                name.bold()
            );
        }
    }
    if let Some(arr) = new.get("smt_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let fname = item
                .get("function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!(
                "   {} [{}] SMT: {}",
                "->".red(),
                finding_codes::SMT_INVARIANT_VIOLATION.bold(),
                fname.bold()
            );
        }
    }
    if let Some(arr) = new.get("sep41_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let fname = item
                .get("function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!(
                "   {} [{}] SEP-41: {}",
                "->".red(),
                finding_codes::SEP41_INTERFACE_DEVIATION.bold(),
                fname.bold()
            );
        }
    }
    if let Some(arr) = new.get("event_issues").and_then(|v| v.as_array()) {
        for item in arr {
            let name = item
                .get("event_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!(
                "   {} [{}] Event: {}",
                "->".red(),
                finding_codes::EVENT_INCONSISTENCY.bold(),
                name.bold()
            );
        }
    }
    if let Some(arr) = new.get("unhandled_results").and_then(|v| v.as_array()) {
        for item in arr {
            let fname = item
                .get("function_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!(
                "   {} [{}] Unhandled result: {}",
                "->".red(),
                finding_codes::UNHANDLED_RESULT.bold(),
                fname.bold()
            );
        }
    }
    if let Some(arr) = new.get("upgrade_reports").and_then(|v| v.as_array()) {
        for rpt in arr {
            if let Some(findings) = rpt.get("findings").and_then(|v| v.as_array()) {
                for item in findings {
                    let msg = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    println!(
                        "   {} [{}] Upgrade: {}",
                        "->".red(),
                        finding_codes::UPGRADE_RISK.bold(),
                        msg
                    );
                }
            }
        }
    }
    if let Some(arr) = new
        .get("vulnerability_db_matches")
        .and_then(|v| v.as_array())
    {
        for item in arr {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let sev = item.get("severity").and_then(|v| v.as_str()).unwrap_or("");
            println!("   {} [VULN-DB] {} ({})", "->".red(), name.bold(), sev);
        }
    }
    if let Some(arr) = new.get("custom_rules").and_then(|v| v.as_array()) {
        for item in arr {
            let name = item.get("rule_name").and_then(|v| v.as_str()).unwrap_or("");
            println!("   {} [CUSTOM] {}", "->".red(), name.bold());
        }
    }
}
