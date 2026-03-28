//! WebAssembly bindings for the Sanctifier static-analysis engine.
//!
//! Compiled with `wasm-pack build --target web` this crate produces the
//! `@sanctifier/wasm` npm package consumed by the frontend dashboard.
//!
//! # Exported functions
//!
//! * [`analyze`] — run all analysis passes with default config.
//! * [`analyze_with_config`] — run with a JSON-serialised [`SanctifyConfig`].

use sanctifier_core::{
    finding_codes, Analyzer, ArithmeticIssue, AuthGapIssue, EventIssue, PanicIssue, SanctifyConfig,
    SizeWarning, StorageCollisionIssue, UnhandledResultIssue, UnsafePattern,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

// Improve panic messages in the browser console.
fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// ── Output types ──────────────────────────────────────────────────────────────

/// A single finding emitted by any analysis pass, normalised for JS consumers.
#[derive(Serialize)]
pub struct Finding {
    /// Canonical code (`S000`–`S012`).
    pub code: &'static str,
    /// Broad category string (matches the finding-code catalogue).
    pub category: &'static str,
    /// Human-readable description of the issue.
    pub message: String,
    /// Source location string when available (e.g. `"function_name:line"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Top-level result returned by [`analyze`] and [`analyze_with_config`].
#[derive(Serialize)]
pub struct AnalysisResult {
    /// Flat list of all findings across every analysis pass.
    pub findings: Vec<Finding>,
    /// Pre-computed counts so JS consumers don't have to iterate.
    pub summary: Summary,
}

/// Aggregate counts included in every [`AnalysisResult`].
#[derive(Serialize)]
pub struct Summary {
    pub total: usize,
    pub auth_gaps: usize,
    pub panic_issues: usize,
    pub arithmetic_issues: usize,
    pub size_warnings: usize,
    pub unsafe_patterns: usize,
    pub storage_collisions: usize,
    pub event_issues: usize,
    pub unhandled_results: usize,
    pub upgrade_risks: usize,
    pub sep41_issues: usize,
    pub has_critical: bool,
    pub has_high: bool,
}

// ── Helpers to convert core types into Finding ───────────────────────────────

fn auth_gap_finding(issue: &AuthGapIssue) -> Finding {
    Finding {
        code: finding_codes::AUTH_GAP,
        category: "authentication",
        message: format!("Missing authentication guard in `{}`", issue.function_name),
        location: Some(issue.function_name.clone()),
    }
}

fn panic_finding(p: &PanicIssue) -> Finding {
    Finding {
        code: finding_codes::PANIC_USAGE,
        category: "panic_handling",
        message: format!("`{}` usage in `{}`", p.issue_type, p.function_name),
        location: Some(p.location.clone()),
    }
}

fn arithmetic_finding(a: &ArithmeticIssue) -> Finding {
    Finding {
        code: finding_codes::ARITHMETIC_OVERFLOW,
        category: "arithmetic",
        message: format!(
            "Unchecked `{}` in `{}` — {}",
            a.operation, a.function_name, a.suggestion
        ),
        location: Some(a.location.clone()),
    }
}

fn size_finding(w: &SizeWarning) -> Finding {
    Finding {
        code: finding_codes::LEDGER_SIZE_RISK,
        category: "storage_limits",
        message: format!(
            "`{}` estimated size {}B approaches/exceeds ledger limit {}B",
            w.struct_name, w.estimated_size, w.limit
        ),
        location: None,
    }
}

fn unsafe_finding(p: &UnsafePattern) -> Finding {
    Finding {
        code: finding_codes::UNSAFE_PATTERN,
        category: "unsafe_patterns",
        message: format!("{:?} at line {}: {}", p.pattern_type, p.line, p.snippet),
        location: Some(format!("line:{}", p.line)),
    }
}

fn collision_finding(c: &StorageCollisionIssue) -> Finding {
    Finding {
        code: finding_codes::STORAGE_COLLISION,
        category: "storage_keys",
        message: c.message.clone(),
        location: Some(c.location.clone()),
    }
}

fn event_finding(e: &EventIssue) -> Finding {
    Finding {
        code: finding_codes::EVENT_INCONSISTENCY,
        category: "events",
        message: e.message.clone(),
        location: Some(e.location.clone()),
    }
}

fn unhandled_finding(r: &UnhandledResultIssue) -> Finding {
    Finding {
        code: finding_codes::UNHANDLED_RESULT,
        category: "logic",
        message: r.message.clone(),
        location: Some(r.location.clone()),
    }
}

// ── Core analysis logic ───────────────────────────────────────────────────────

fn run_analysis(analyzer: &Analyzer, source: &str) -> AnalysisResult {
    let auth_gaps = analyzer.scan_auth_gaps(source);
    let panic_issues = analyzer.scan_panics(source);
    let arithmetic_issues = analyzer.scan_arithmetic_overflow(source);
    let size_warnings = analyzer.analyze_ledger_size(source);
    let unsafe_patterns = analyzer.analyze_unsafe_patterns(source);
    let storage_collisions = analyzer.scan_storage_collisions(source);
    let event_issues = analyzer.scan_events(source);
    let unhandled_results = analyzer.scan_unhandled_results(source);
    let upgrade_report = analyzer.analyze_upgrade_patterns(source);
    let sep41_report = analyzer.verify_sep41_interface(source);

    let mut findings: Vec<Finding> = Vec::new();

    for g in &auth_gaps {
        findings.push(auth_gap_finding(g));
    }
    for p in &panic_issues {
        findings.push(panic_finding(p));
    }
    for a in &arithmetic_issues {
        findings.push(arithmetic_finding(a));
    }
    for w in &size_warnings {
        findings.push(size_finding(w));
    }
    for p in &unsafe_patterns {
        findings.push(unsafe_finding(p));
    }
    for c in &storage_collisions {
        findings.push(collision_finding(c));
    }
    for e in &event_issues {
        findings.push(event_finding(e));
    }
    for r in &unhandled_results {
        findings.push(unhandled_finding(r));
    }
    for f in &upgrade_report.findings {
        findings.push(Finding {
            code: finding_codes::UPGRADE_RISK,
            category: "upgrades",
            message: f.message.clone(),
            location: Some(f.location.clone()),
        });
    }
    for issue in &sep41_report.issues {
        findings.push(Finding {
            code: finding_codes::SEP41_INTERFACE_DEVIATION,
            category: "token_interface",
            message: issue.message.clone(),
            location: Some(issue.location.clone()),
        });
    }

    let summary = Summary {
        total: findings.len(),
        auth_gaps: auth_gaps.len(),
        panic_issues: panic_issues.len(),
        arithmetic_issues: arithmetic_issues.len(),
        size_warnings: size_warnings.len(),
        unsafe_patterns: unsafe_patterns.len(),
        storage_collisions: storage_collisions.len(),
        event_issues: event_issues.len(),
        unhandled_results: unhandled_results.len(),
        upgrade_risks: upgrade_report.findings.len(),
        sep41_issues: sep41_report.issues.len(),
        has_critical: false, // wasm passes don't produce critical-severity findings
        has_high: !auth_gaps.is_empty() || !upgrade_report.findings.is_empty(),
    };

    AnalysisResult { findings, summary }
}

// ── Public WASM API ───────────────────────────────────────────────────────────

/// Analyse Soroban contract source code with default configuration.
///
/// Returns a JS object shaped as [`AnalysisResult`]:
/// ```json
/// {
///   "findings": [{ "code": "S001", "category": "...", "message": "...", "location": "..." }],
///   "summary":  { "total": 3, "has_critical": false, "has_high": true, ... }
/// }
/// ```
#[wasm_bindgen]
pub fn analyze(source: &str) -> JsValue {
    set_panic_hook();
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let result = run_analysis(&analyzer, source);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Analyse with a JSON-serialised [`SanctifyConfig`].
///
/// Falls back to `SanctifyConfig::default()` if `config_json` cannot be parsed.
#[wasm_bindgen]
pub fn analyze_with_config(config_json: &str, source: &str) -> JsValue {
    set_panic_hook();
    let config: SanctifyConfig = serde_json::from_str(config_json).unwrap_or_default();
    let analyzer = Analyzer::new(config);
    let result = run_analysis(&analyzer, source);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Return the full finding-code catalogue as a JS array.
///
/// Useful for building UI legend tables without hard-coding the codes.
#[wasm_bindgen]
pub fn finding_codes() -> JsValue {
    let codes = sanctifier_core::finding_codes::all_finding_codes();
    serde_wasm_bindgen::to_value(&codes).unwrap_or(JsValue::NULL)
}

/// Return the crate version string (e.g. `"0.1.0"`).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
