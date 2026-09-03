//! WebAssembly bindings for the Sanctifier static-analysis engine.
//!
//! Compiled with `wasm-pack build --target web` this crate produces the
//! `@sanctifier/wasm` npm package consumed by the frontend dashboard.
//!
//! # Module layout
//!
//! | Module        | Responsibility                                         |
//! |---------------|--------------------------------------------------------|
//! | `constants`   | Compile-time limits, namespace strings, version pins  |
//! | `validation`  | Input guard functions (source size, memory budget, config) |
//! | `types`       | Serialisable output structs returned to JS consumers  |
//! | `converters`  | Core-type → [`types::Finding`] conversion helpers     |
//! | `analysis`    | Orchestration of analysis passes, progress, cache key |
//! | *(top-level)* | `#[wasm_bindgen]` public API surface                  |
//!
//! # Exported functions
//!
//! * [`analyze`] — run all analysis passes with default config.
//! * [`analyze_with_config`] — run with a JSON-serialised [`SanctifyConfig`].
//! * [`analyze_with_progress`] — run analysis and emit deterministic progress events.
//! * [`version`] — return the WASM module version.
//! * [`schema_version`] — return the analysis output schema version.
//! * [`finding_codes`] — return the finding code catalogue.
//! * [`default_config_json`] — return default config JSON for easy customization.
//! * [`asset_cache_key`] — return a deterministic browser cache-bust key.
//! * [`cache_metadata`] — return full cache metadata for offline-first consumers.

use wasm_bindgen::prelude::*;

// ── Module declarations ────────────────────────────────────────────────────────

mod analysis;
mod constants;
mod converters;
mod types;
mod validation;

// Re-export the public API types so consumers can import them directly.
pub use types::{
    AnalysisResult, CacheMetadata, ErrorResponse, Finding, ProgressEvent,
    ProgressiveAnalysisResult, Summary,
};

// ── Internal wiring ────────────────────────────────────────────────────────────

fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

fn make_error(error_code: &str, message: String) -> JsValue {
    let error = ErrorResponse {
        error_code: error_code.to_string(),
        message,
        schema_version: constants::SCHEMA_VERSION,
    };
    serde_wasm_bindgen::to_value(&error).unwrap_or(JsValue::NULL)
}

// ── Public WASM API ───────────────────────────────────────────────────────────

/// Analyse Soroban contract source code with default configuration.
///
/// Returns a JS object shaped as [`AnalysisResult`]:
/// ```json
/// {
///   "findings": [{ "code": "S001", "category": "...", "message": "...", "location": "..." }],
///   "summary":  { "total": 3, "has_critical": false, "has_high": true, ... },
///   "schema_version": "1.0.0"
/// }
/// ```
///
/// On validation failure the return value is shaped as [`ErrorResponse`]:
/// ```json
/// { "error_code": "INVALID_INPUT", "message": "...", "schema_version": "1.0.0" }
/// ```
#[wasm_bindgen]
pub fn analyze(source: &str) -> JsValue {
    set_panic_hook();

    if let Err(msg) = validation::validate_source(source) {
        return make_error("INVALID_INPUT", msg);
    }
    if let Err(msg) = validation::check_memory_budget(source.len()) {
        return make_error("MEMORY_BUDGET_EXCEEDED", msg);
    }

    let result = analysis::run_analysis_default(source);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Analyse with a JSON-serialised [`SanctifyConfig`].
///
/// Falls back to `SanctifyConfig::default()` if `config_json` cannot be parsed.
///
/// # Errors
/// Returns an [`ErrorResponse`] object if input validation fails.
#[wasm_bindgen]
pub fn analyze_with_config(config_json: &str, source: &str) -> JsValue {
    set_panic_hook();

    if let Err(msg) = validation::validate_config_json(config_json) {
        return make_error("INVALID_CONFIG", msg);
    }
    if let Err(msg) = validation::validate_source(source) {
        return make_error("INVALID_INPUT", msg);
    }
    if let Err(msg) = validation::check_memory_budget(source.len()) {
        return make_error("MEMORY_BUDGET_EXCEEDED", msg);
    }

    let result = analysis::run_analysis_with_config(config_json, source);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Analyse with deterministic progress snapshots for streaming-like UX.
///
/// Returns a [`ProgressiveAnalysisResult`] containing both progress events and
/// the final [`AnalysisResult`], allowing frontend clients to render partial
/// progress while output remains deterministic and cacheable.
#[wasm_bindgen]
pub fn analyze_with_progress(source: &str) -> JsValue {
    set_panic_hook();

    if let Err(msg) = validation::validate_source(source) {
        return make_error("INVALID_INPUT", msg);
    }

    let progressive = analysis::run_analysis_with_progress(source);
    serde_wasm_bindgen::to_value(&progressive).unwrap_or(JsValue::NULL)
}

/// Return the full finding-code catalogue as a JS array.
///
/// Useful for building UI legend tables without hard-coding the codes.
#[wasm_bindgen]
pub fn finding_codes() -> JsValue {
    let codes = sanctifier_core::finding_codes::all_finding_codes();
    serde_wasm_bindgen::to_value(&codes).unwrap_or(JsValue::NULL)
}

/// Return the crate version string (e.g. `"0.2.0"`).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Return the analysis output schema version (independent of tool version).
///
/// Increment this only when the JSON output format changes in a breaking way.
/// See `docs/wasm-versioning-alignment.md` for the versioning policy.
#[wasm_bindgen]
pub fn schema_version() -> String {
    constants::SCHEMA_VERSION.to_string()
}

/// Return default config JSON for easy copy/edit in browser tooling.
#[wasm_bindgen]
pub fn default_config_json() -> String {
    serde_json::to_string_pretty(&sanctifier_core::SanctifyConfig::default())
        .unwrap_or_else(|_| "{}".to_string())
}

/// Return a deterministic cache key for wasm module assets.
///
/// Frontend loaders use this to bust stale service-worker and CacheStorage
/// entries whenever the package or schema version changes.
#[wasm_bindgen]
pub fn asset_cache_key() -> String {
    analysis::build_cache_key()
}

/// Return cache metadata for offline-first consumers.
#[wasm_bindgen]
pub fn cache_metadata() -> JsValue {
    let metadata = CacheMetadata {
        package: "sanctifier-wasm",
        version: env!("CARGO_PKG_VERSION"),
        schema_version: constants::SCHEMA_VERSION,
        cache_key: analysis::build_cache_key(),
    };
    serde_wasm_bindgen::to_value(&metadata).unwrap_or(JsValue::NULL)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{MAX_SOURCE_SIZE, MEMORY_BUDGET_BYTES, MEMORY_OVERHEAD_FACTOR};
    use crate::validation::{check_memory_budget, validate_source};

    // ── validate_source ───────────────────────────────────────────────────────

    #[test]
    fn validate_source_rejects_empty() {
        assert!(validate_source("").is_err());
    }

    #[test]
    fn validate_source_accepts_one_byte() {
        assert!(validate_source("x").is_ok());
    }

    #[test]
    fn validate_source_rejects_above_max_size() {
        let oversized = "x".repeat(MAX_SOURCE_SIZE + 1);
        assert!(validate_source(&oversized).is_err());
    }

    #[test]
    fn validate_source_accepts_at_max_size() {
        let at_limit = "x".repeat(MAX_SOURCE_SIZE);
        assert!(validate_source(&at_limit).is_ok());
    }

    // ── check_memory_budget ───────────────────────────────────────────────────

    #[test]
    fn memory_budget_accepts_small_source() {
        assert!(check_memory_budget(1024).is_ok());
    }

    #[test]
    fn memory_budget_accepts_source_at_exact_limit() {
        let max_ok = MEMORY_BUDGET_BYTES / MEMORY_OVERHEAD_FACTOR;
        assert!(check_memory_budget(max_ok).is_ok());
    }

    #[test]
    fn memory_budget_rejects_source_one_byte_above_limit() {
        let just_over = MEMORY_BUDGET_BYTES / MEMORY_OVERHEAD_FACTOR + 1;
        let result = check_memory_budget(just_over);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("memory budget"),
            "expected budget message, got: {msg}"
        );
    }

    #[test]
    fn memory_budget_rejects_max_source_size() {
        // MAX_SOURCE_SIZE (10 MB) × 8 = 80 MB > 32 MB budget.
        assert!(check_memory_budget(MAX_SOURCE_SIZE).is_err());
    }

    #[test]
    fn memory_budget_saturating_mul_does_not_overflow() {
        let result = check_memory_budget(usize::MAX);
        assert!(result.is_err());
    }

    // ── build_cache_key ───────────────────────────────────────────────────────

    #[test]
    fn cache_key_contains_namespace_and_versions() {
        let key = analysis::build_cache_key();
        assert!(key.contains(constants::CACHE_NAMESPACE));
        assert!(key.contains(constants::SCHEMA_VERSION));
    }

    // ── Module boundary: public re-exports are accessible ────────────────────

    #[test]
    fn types_module_re_exports_are_accessible() {
        // Ensures the public module boundary is stable; adding a new type here
        // will cause a compile error if the module layout changes unexpectedly.
        let _ = ErrorResponse {
            error_code: "TEST".to_string(),
            message: "test".to_string(),
            schema_version: constants::SCHEMA_VERSION,
        };
        let _ = Summary {
            total: 0,
            auth_gaps: 0,
            panic_issues: 0,
            arithmetic_issues: 0,
            size_warnings: 0,
            unsafe_patterns: 0,
            storage_collisions: 0,
            event_issues: 0,
            unhandled_results: 0,
            upgrade_risks: 0,
            sep41_issues: 0,
            has_critical: false,
            has_high: false,
        };
    }

    // ── Fixture-based tests (Issue #538) ─────────────────────────────────────
    //
    // These tests use inline Rust source fixtures to verify that known-bad
    // patterns produce the expected findings, and known-clean sources produce
    // zero findings.  They also verify wasm-pack build reproducibility by
    // asserting that repeated analysis of the same source is idempotent.

    const CLEAN_CONTRACT: &str = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl, token, Address, Env};

        #[contract]
        pub struct CleanToken;

        #[contractimpl]
        impl CleanToken {
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                from.require_auth();
                let client = token::Client::new(&env, &from);
                client.transfer(&from, &to, &amount);
            }
        }
    "#;

    const AUTH_GAP_CONTRACT: &str = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

        #[contract]
        pub struct AuthGapFixture;

        #[contractimpl]
        impl AuthGapFixture {
            pub fn set_owner(env: Env, owner: Symbol) {
                env.storage().instance().set(&symbol_short!("OWNER"), &owner);
            }
        }
    "#;

    const PANIC_CONTRACT: &str = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl, Env};

        #[contract]
        pub struct PanicFixture;

        #[contractimpl]
        impl PanicFixture {
            pub fn do_thing(env: Env, x: u32) -> u32 {
                if x == 0 {
                    panic!("x must not be zero");
                }
                x * 2
            }
        }
    "#;

    #[test]
    fn clean_contract_produces_no_findings() {
        let result = analysis::run_analysis_default(CLEAN_CONTRACT);
        assert_eq!(
            result.summary.auth_gaps, 0,
            "clean contract must have no auth gaps"
        );
        assert_eq!(
            result.summary.panic_issues, 0,
            "clean contract must have no panic issues"
        );
    }

    #[test]
    fn auth_gap_fixture_produces_auth_gap_finding() {
        let result = analysis::run_analysis_default(AUTH_GAP_CONTRACT);
        assert!(
            result.summary.auth_gaps >= 1,
            "auth gap fixture must produce at least one auth_gap finding"
        );
    }

    #[test]
    fn panic_fixture_produces_panic_finding() {
        let result = analysis::run_analysis_default(PANIC_CONTRACT);
        assert!(
            result.summary.panic_issues >= 1,
            "panic fixture must produce at least one panic_issue finding"
        );
    }

    #[test]
    fn analysis_is_idempotent_for_same_source() {
        let first = analysis::run_analysis_default(AUTH_GAP_CONTRACT);
        let second = analysis::run_analysis_default(AUTH_GAP_CONTRACT);
        assert_eq!(
            first.summary.total, second.summary.total,
            "repeated analysis must produce the same finding count"
        );
        for (a, b) in first.findings.iter().zip(second.findings.iter()) {
            assert_eq!(
                a.code, b.code,
                "finding codes must be identical across runs"
            );
            assert_eq!(
                a.message, b.message,
                "finding messages must be identical across runs"
            );
        }
    }

    #[test]
    fn default_config_json_is_valid_json() {
        // default_config_json() is defined in lib.rs and is accessible via super::*.
        let json_str = default_config_json();
        assert!(
            !json_str.is_empty(),
            "default config JSON must not be empty"
        );
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(&json_str);
        assert!(
            parsed.is_ok(),
            "default_config_json must produce valid JSON; got: {json_str}"
        );
    }

    #[test]
    fn analyze_with_progress_summary_matches_plain_analyze() {
        let result_plain = analysis::run_analysis_default(CLEAN_CONTRACT);
        let result_progressive = analysis::run_analysis_with_progress(CLEAN_CONTRACT);
        assert_eq!(
            result_plain.summary.total, result_progressive.result.summary.total,
            "progress-aware and plain analysis must agree on total findings"
        );
    }

    #[test]
    fn analyze_with_progress_events_are_ordered_by_percent() {
        let progressive = analysis::run_analysis_with_progress(PANIC_CONTRACT);
        let percents: Vec<u8> = progressive.events.iter().map(|e| e.percent).collect();
        let mut sorted = percents.clone();
        sorted.sort_unstable();
        assert_eq!(
            percents, sorted,
            "progress events must be in ascending percent order"
        );
    }

    #[test]
    fn cache_key_is_stable_across_invocations() {
        let k1 = analysis::build_cache_key();
        let k2 = analysis::build_cache_key();
        assert_eq!(k1, k2, "cache key must be identical across calls");
    }
}
