//! Compile-time constants for the WASM package.
//!
//! All tuneable limits and namespace strings live here so they can be
//! referenced by both the implementation modules and the test suite without
//! coupling either to internal details of the other.

/// Analysis output schema version (independent of tool version).
///
/// Increment only when the JSON output format changes in a breaking way.
/// See `docs/wasm-versioning-alignment.md` for the full versioning policy.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Maximum allowed source code size (10 MB).
pub const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024;

/// Minimum required source code size (1 byte).
pub const MIN_SOURCE_SIZE: usize = 1;

/// Conservative per-invocation memory budget (32 MB).
///
/// WASM32 has a 4 GB virtual address space but the default linear memory
/// grows in 64 KB pages.  Capping working-set estimation to 32 MB prevents
/// runaway allocations from stalling the browser tab.
pub const MEMORY_BUDGET_BYTES: usize = 32 * 1024 * 1024;

/// Conservative overhead multiplier for memory budget estimation.
///
/// The analyser expands source into several internal representations (tokens,
/// AST nodes, finding lists).  A factor of 8 covers the worst-case observed
/// peak without live heap profiling.
pub const MEMORY_OVERHEAD_FACTOR: usize = 8;

/// Maximum allowed configuration JSON size (1 MB).
pub const MAX_CONFIG_SIZE: usize = 1024 * 1024;

/// Namespace prefix for browser-side wasm asset caches.
pub const CACHE_NAMESPACE: &str = "sanctifier-wasm";

// ── Target-specific memory budgets ────────────────────────────────────────────
//
// Node.js WASM has access to the full V8 heap (often 1–2 GB in practice),
// while browser tabs are typically limited to ~512 MB before the OS OOM-kills
// the tab.  We apply a more conservative budget for browser targets so the
// tab stays responsive, and a higher budget for Node targets to match CI usage.

/// Conservative per-invocation memory budget for **browser** WASM targets (32 MB).
pub const MEMORY_BUDGET_BYTES_BROWSER: usize = 32 * 1024 * 1024;

/// Per-invocation memory budget for **Node.js** WASM targets (128 MB).
///
/// Node has access to a much larger V8 heap than a browser tab; a higher budget
/// allows analysis of larger contracts without false `MEMORY_BUDGET_EXCEEDED` errors
/// in CI or server-side workflows.
pub const MEMORY_BUDGET_BYTES_NODE: usize = 128 * 1024 * 1024;

/// Maximum source size for **browser** targets (10 MB, same as core).
pub const MAX_SOURCE_SIZE_BROWSER: usize = MAX_SOURCE_SIZE;

/// Maximum source size for **Node.js** targets (25 MB).
///
/// Node pipelines (CI, server-side scan APIs) often process larger generated
/// or concatenated contract files that a browser user would never upload.
pub const MAX_SOURCE_SIZE_NODE: usize = 25 * 1024 * 1024;
