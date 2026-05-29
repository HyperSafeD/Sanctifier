//! Analysis orchestration: runs all passes and builds output structures.
//!
//! This module owns the logic for driving `sanctifier-core`, collecting
//! findings from every pass, assembling the final `AnalysisResult`, and
//! constructing progress events and cache keys.  It does not perform any
//! input validation (see [`crate::validation`]) or JS serialisation (see the
//! top-level WASM API in `lib.rs`).

use sanctifier_core::{finding_codes, Analyzer, SanctifyConfig};

use crate::constants::{CACHE_NAMESPACE, SCHEMA_VERSION};
use crate::converters;
use crate::types::{AnalysisResult, Finding, ProgressEvent, ProgressiveAnalysisResult, Summary};

// ── Progress phase table ───────────────────────────────────────────────────────

const PROGRESS_PHASES: [(&str, u8); 5] = [
    ("Validating source input", 10),
    ("Parsing and indexing contract", 30),
    ("Running security passes", 60),
    ("Aggregating findings", 85),
    ("Finalizing schema output", 100),
];

// ── Internal helpers ───────────────────────────────────────────────────────────

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
        findings.push(converters::auth_gap(g));
    }
    for p in &panic_issues {
        findings.push(converters::panic_issue(p));
    }
    for a in &arithmetic_issues {
        findings.push(converters::arithmetic(a));
    }
    for w in &size_warnings {
        findings.push(converters::size_warning(w));
    }
    for p in &unsafe_patterns {
        findings.push(converters::unsafe_pattern(p));
    }
    for c in &storage_collisions {
        findings.push(converters::storage_collision(c));
    }
    for e in &event_issues {
        findings.push(converters::event_issue(e));
    }
    for r in &unhandled_results {
        findings.push(converters::unhandled_result(r));
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
        has_critical: false,
        has_high: !auth_gaps.is_empty() || !upgrade_report.findings.is_empty(),
    };

    AnalysisResult {
        findings,
        summary,
        schema_version: SCHEMA_VERSION,
    }
}

fn build_progress_events(total_findings: usize) -> Vec<ProgressEvent> {
    PROGRESS_PHASES
        .iter()
        .enumerate()
        .map(|(idx, (phase, percent))| ProgressEvent {
            phase,
            percent: *percent,
            findings_so_far: ((idx + 1) * total_findings) / PROGRESS_PHASES.len(),
        })
        .collect()
}

// ── Public API (called from lib.rs) ───────────────────────────────────────────

/// Run all analysis passes with `SanctifyConfig::default()`.
pub fn run_analysis_default(source: &str) -> AnalysisResult {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    run_analysis(&analyzer, source)
}

/// Run all analysis passes, deserialising config from JSON (falls back to
/// `SanctifyConfig::default()` if parsing fails).
pub fn run_analysis_with_config(config_json: &str, source: &str) -> AnalysisResult {
    let config: SanctifyConfig = serde_json::from_str(config_json).unwrap_or_default();
    let analyzer = Analyzer::new(config);
    run_analysis(&analyzer, source)
}

/// Run all passes and bundle the result with deterministic progress events.
pub fn run_analysis_with_progress(source: &str) -> ProgressiveAnalysisResult {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let result = run_analysis(&analyzer, source);
    let events = build_progress_events(result.summary.total);
    ProgressiveAnalysisResult { events, result }
}

/// Return a deterministic cache-bust key (`namespace:pkg_version:schema_version`).
pub fn build_cache_key() -> String {
    format!(
        "{}:{}:{}",
        CACHE_NAMESPACE,
        env!("CARGO_PKG_VERSION"),
        SCHEMA_VERSION
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{CACHE_NAMESPACE, SCHEMA_VERSION};

    #[test]
    fn cache_key_contains_namespace_and_schema_version() {
        let key = build_cache_key();
        assert!(key.contains(CACHE_NAMESPACE));
        assert!(key.contains(SCHEMA_VERSION));
    }

    #[test]
    fn cache_key_has_three_colon_delimited_segments() {
        let key = build_cache_key();
        assert_eq!(key.splitn(3, ':').count(), 3);
    }

    #[test]
    fn progress_events_match_phase_count() {
        let events = build_progress_events(10);
        assert_eq!(events.len(), PROGRESS_PHASES.len());
    }

    #[test]
    fn progress_events_end_at_100_percent() {
        let events = build_progress_events(5);
        assert_eq!(events.last().unwrap().percent, 100);
    }

    #[test]
    fn run_analysis_default_produces_schema_version() {
        let result = run_analysis_default("fn foo() {}");
        assert_eq!(result.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn run_analysis_with_invalid_config_falls_back_to_defaults() {
        // Invalid JSON → falls back to default, should not panic.
        let result = run_analysis_with_config("{invalid}", "fn foo() {}");
        assert_eq!(result.schema_version, SCHEMA_VERSION);
    }
}
