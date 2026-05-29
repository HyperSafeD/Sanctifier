//! Serialisable output structs returned to JS consumers.
//!
//! All types implement [`serde::Serialize`] so they can be converted to
//! `JsValue` via `serde-wasm-bindgen`.  They are intentionally decoupled from
//! the `sanctifier-core` types so the WASM public API surface can evolve
//! independently of internal analyser structures.

use serde::Serialize;

/// Error response for validation or processing failures.
///
/// ```json
/// { "error_code": "INVALID_INPUT", "message": "...", "schema_version": "1.0.0" }
/// ```
#[derive(Serialize)]
pub struct ErrorResponse {
    /// Machine-readable code (e.g. `"INVALID_INPUT"`, `"PARSE_ERROR"`).
    pub error_code: String,
    /// Human-readable description forwarded to the JS caller.
    pub message: String,
    /// Schema version for response envelope consistency.
    pub schema_version: &'static str,
}

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

/// Top-level result returned by [`analyze`](crate::analyze) and
/// [`analyze_with_config`](crate::analyze_with_config).
#[derive(Serialize)]
pub struct AnalysisResult {
    /// Flat list of all findings across every analysis pass.
    pub findings: Vec<Finding>,
    /// Pre-computed counts so JS consumers don't have to iterate.
    pub summary: Summary,
    /// Schema version for versioning alignment.
    pub schema_version: &'static str,
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

/// Progress event emitted by
/// [`analyze_with_progress`](crate::analyze_with_progress).
#[derive(Serialize)]
pub struct ProgressEvent {
    /// Phase label shown in the UI (e.g. `"Running security passes"`).
    pub phase: &'static str,
    /// Completion percentage `[0, 100]`.
    pub percent: u8,
    /// Running finding count at this phase boundary.
    pub findings_so_far: usize,
}

/// Progressive response for browsers rendering partial progress.
#[derive(Serialize)]
pub struct ProgressiveAnalysisResult {
    /// Ordered sequence of phase checkpoints emitted before the final result.
    pub events: Vec<ProgressEvent>,
    /// Full analysis result available once all phases complete.
    pub result: AnalysisResult,
}

/// Minimal metadata required by browser cache layers.
#[derive(Serialize)]
pub struct CacheMetadata {
    /// npm package name (`"sanctifier-wasm"`).
    pub package: &'static str,
    /// Crate semver version.
    pub version: &'static str,
    /// Analysis output schema version.
    pub schema_version: &'static str,
    /// Deterministic cache-bust key derived from package + schema versions.
    pub cache_key: String,
}
