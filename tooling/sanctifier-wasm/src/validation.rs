//! Input guard functions for the WASM public API.
//!
//! Each function returns `Ok(())` on success or `Err(String)` with a
//! human-readable message that is forwarded directly to JS callers via
//! [`ErrorResponse`](crate::types::ErrorResponse).

use crate::constants::{
    MAX_CONFIG_SIZE, MAX_SOURCE_SIZE, MAX_SOURCE_SIZE_BROWSER, MAX_SOURCE_SIZE_NODE,
    MEMORY_BUDGET_BYTES, MEMORY_BUDGET_BYTES_BROWSER, MEMORY_BUDGET_BYTES_NODE,
    MEMORY_OVERHEAD_FACTOR, MIN_SOURCE_SIZE,
};

/// Deployment target for target-aware validation.
///
/// Pass this to [`validate_for_target`] to apply limits that match the
/// environment where the WASM module is actually running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmTarget {
    /// Running inside a browser tab (conservative memory limits).
    Browser,
    /// Running inside Node.js (e.g. CI, server-side API, test suite).
    Node,
}

/// Validate source code input against size limits.
///
/// # Errors
/// - `"Source code cannot be empty"` — when `source` is zero bytes.
/// - `"Source code exceeds maximum size of N bytes (got M bytes)"` — when
///   `source.len() > MAX_SOURCE_SIZE`.
pub fn validate_source(source: &str) -> Result<(), String> {
    let len = source.len();

    if len < MIN_SOURCE_SIZE {
        return Err("Source code cannot be empty".to_string());
    }

    if len > MAX_SOURCE_SIZE {
        return Err(format!(
            "Source code exceeds maximum size of {} bytes (got {} bytes)",
            MAX_SOURCE_SIZE, len
        ));
    }

    Ok(())
}

/// Estimate worst-case heap usage and verify it fits within `MEMORY_BUDGET_BYTES`.
///
/// This is a pre-flight check — it fires *before* any allocation so the WASM
/// linear memory is never exhausted silently.
///
/// # Errors
/// Returns a `"MEMORY_BUDGET_EXCEEDED"` message when the estimated working set
/// (`source_len × MEMORY_OVERHEAD_FACTOR`) exceeds `MEMORY_BUDGET_BYTES`.
pub fn check_memory_budget(source_len: usize) -> Result<(), String> {
    let estimated = source_len.saturating_mul(MEMORY_OVERHEAD_FACTOR);
    if estimated > MEMORY_BUDGET_BYTES {
        return Err(format!(
            "Estimated working set {} bytes exceeds memory budget of {} bytes. \
             Split the contract into smaller files.",
            estimated, MEMORY_BUDGET_BYTES
        ));
    }
    Ok(())
}

/// Validate configuration JSON against size limits.
///
/// An empty or whitespace-only string is accepted (caller falls back to defaults).
///
/// # Errors
/// - `"Configuration JSON exceeds maximum size of 1 MB"` — when
///   `config_json.len() > MAX_CONFIG_SIZE`.
pub fn validate_config_json(config_json: &str) -> Result<(), String> {
    if config_json.trim().is_empty() {
        return Ok(());
    }

    if config_json.len() > MAX_CONFIG_SIZE {
        return Err(format!(
            "Configuration JSON exceeds maximum size of {} bytes",
            MAX_CONFIG_SIZE
        ));
    }

    Ok(())
}

/// Validate source code and memory budget for a specific deployment target.
///
/// Applies target-appropriate size and memory limits so that:
/// - Browser callers receive conservative limits that protect tab stability.
/// - Node.js callers receive relaxed limits suitable for CI / server workflows.
///
/// # Errors
/// Returns the first validation error encountered:
/// - `"Source code cannot be empty"` — source is zero bytes.
/// - `"Source code exceeds maximum size of N bytes (got M bytes)"` — above limit.
/// - Memory budget exceeded message — estimated working set too large.
pub fn validate_for_target(source: &str, target: WasmTarget) -> Result<(), String> {
    let len = source.len();

    let (max_source, memory_budget) = match target {
        WasmTarget::Browser => (MAX_SOURCE_SIZE_BROWSER, MEMORY_BUDGET_BYTES_BROWSER),
        WasmTarget::Node => (MAX_SOURCE_SIZE_NODE, MEMORY_BUDGET_BYTES_NODE),
    };

    if len < MIN_SOURCE_SIZE {
        return Err("Source code cannot be empty".to_string());
    }

    if len > max_source {
        return Err(format!(
            "Source code exceeds maximum size of {} bytes (got {} bytes)",
            max_source, len
        ));
    }

    let estimated = len.saturating_mul(MEMORY_OVERHEAD_FACTOR);
    if estimated > memory_budget {
        return Err(format!(
            "Estimated working set {} bytes exceeds {} memory budget of {} bytes. \
             Split the contract into smaller files.",
            estimated,
            match target {
                WasmTarget::Browser => "browser",
                WasmTarget::Node => "node",
            },
            memory_budget
        ));
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{MAX_SOURCE_SIZE, MEMORY_BUDGET_BYTES, MEMORY_OVERHEAD_FACTOR};

    #[test]
    fn validate_source_empty_returns_err() {
        assert!(validate_source("").is_err());
    }

    #[test]
    fn validate_source_single_byte_ok() {
        assert!(validate_source("x").is_ok());
    }

    #[test]
    fn validate_source_at_max_size_ok() {
        assert!(validate_source(&"x".repeat(MAX_SOURCE_SIZE)).is_ok());
    }

    #[test]
    fn validate_source_one_over_max_err() {
        let result = validate_source(&"x".repeat(MAX_SOURCE_SIZE + 1));
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("exceeds maximum size"));
    }

    #[test]
    fn memory_budget_small_source_ok() {
        assert!(check_memory_budget(1024).is_ok());
    }

    #[test]
    fn memory_budget_exact_limit_ok() {
        let max_ok = MEMORY_BUDGET_BYTES / MEMORY_OVERHEAD_FACTOR;
        assert!(check_memory_budget(max_ok).is_ok());
    }

    #[test]
    fn memory_budget_one_over_limit_err() {
        let just_over = MEMORY_BUDGET_BYTES / MEMORY_OVERHEAD_FACTOR + 1;
        let result = check_memory_budget(just_over);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("memory budget"));
    }

    #[test]
    fn memory_budget_usize_max_does_not_overflow() {
        assert!(check_memory_budget(usize::MAX).is_err());
    }

    #[test]
    fn validate_config_empty_string_ok() {
        assert!(validate_config_json("").is_ok());
    }

    #[test]
    fn validate_config_whitespace_only_ok() {
        assert!(validate_config_json("   ").is_ok());
    }

    #[test]
    fn validate_config_oversized_err() {
        let big = "x".repeat(MAX_CONFIG_SIZE + 1);
        let result = validate_config_json(&big);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum size"));
    }

    #[test]
    fn validate_config_at_max_size_ok() {
        assert!(validate_config_json(&"x".repeat(MAX_CONFIG_SIZE)).is_ok());
    }

    // ── validate_for_target (node vs browser parity) ──────────────────────────

    #[test]
    fn target_browser_rejects_empty() {
        assert!(validate_for_target("", WasmTarget::Browser).is_err());
    }

    #[test]
    fn target_node_rejects_empty() {
        assert!(validate_for_target("", WasmTarget::Node).is_err());
    }

    #[test]
    fn target_browser_accepts_small_source() {
        assert!(validate_for_target("fn foo() {}", WasmTarget::Browser).is_ok());
    }

    #[test]
    fn target_node_accepts_small_source() {
        assert!(validate_for_target("fn foo() {}", WasmTarget::Node).is_ok());
    }

    #[test]
    fn target_node_accepts_larger_source_than_browser() {
        // A source slightly above the browser limit but below the node limit.
        let mid_size = "x".repeat(MAX_SOURCE_SIZE_BROWSER + 1);
        assert!(
            validate_for_target(&mid_size, WasmTarget::Browser).is_err(),
            "browser should reject source above its limit"
        );
        assert!(
            validate_for_target(&mid_size, WasmTarget::Node).is_ok(),
            "node should accept source within its higher limit"
        );
    }

    #[test]
    fn target_node_memory_budget_is_higher_than_browser() {
        // Compile-time check: node budget must exceed browser budget
        const _: () = assert!(MEMORY_BUDGET_BYTES_NODE > MEMORY_BUDGET_BYTES_BROWSER);
    }

    #[test]
    fn target_browser_memory_error_mentions_browser() {
        let just_over = MEMORY_BUDGET_BYTES_BROWSER / MEMORY_OVERHEAD_FACTOR + 1;
        let source = "x".repeat(just_over);
        let err = validate_for_target(&source, WasmTarget::Browser).unwrap_err();
        assert!(
            err.contains("browser"),
            "error should mention target: {err}"
        );
    }

    #[test]
    fn target_node_memory_error_mentions_node() {
        let just_over = MEMORY_BUDGET_BYTES_NODE / MEMORY_OVERHEAD_FACTOR + 1;
        let source = "x".repeat(just_over);
        let err = validate_for_target(&source, WasmTarget::Node).unwrap_err();
        assert!(err.contains("node"), "error should mention target: {err}");
    }
}
