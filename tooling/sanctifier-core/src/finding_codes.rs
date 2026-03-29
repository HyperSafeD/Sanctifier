//! Canonical finding codes emitted by Sanctifier analysis passes.
//!
//! Each constant (`S000` – `S013`) maps to a single diagnostic category.
//! Call `all_finding_codes()` to retrieve the full catalogue at runtime.

use serde::{Deserialize, Serialize};

/// Severity level for findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FindingSeverity {
    /// Critical severity - immediate security risk
    Critical,
    /// High severity - significant security concern
    High,
    /// Medium severity - potential issue
    #[default]
    Medium,
    /// Low severity - minor concern
    Low,
    /// Informational - no immediate risk
    Info,
}

/// Analysis timed out for a file (see `--timeout`).
pub const ANALYSIS_TIMEOUT: &str = "S000";
/// Missing authentication guard in a privileged function.
pub const AUTH_GAP: &str = "S001";
/// `panic!` / `unwrap` / `expect` usage that may abort.
pub const PANIC_USAGE: &str = "S002";
/// Unchecked arithmetic with overflow / underflow risk.
pub const ARITHMETIC_OVERFLOW: &str = "S003";
/// Ledger entry size exceeds or approaches the configured limit.
pub const LEDGER_SIZE_RISK: &str = "S004";
/// Potential storage-key collision across data paths.
pub const STORAGE_COLLISION: &str = "S005";
/// Potentially unsafe language / runtime pattern.
pub const UNSAFE_PATTERN: &str = "S006";
/// User-defined custom rule matched contract source.
pub const CUSTOM_RULE_MATCH: &str = "S007";
/// Inconsistent topic counts or sub-optimal gas patterns in events.
pub const EVENT_INCONSISTENCY: &str = "S008";
/// A `Result` return value is not consumed or handled.
pub const UNHANDLED_RESULT: &str = "S009";
/// Potential security risk in contract upgrade / admin mechanisms.
pub const UPGRADE_RISK: &str = "S010";
/// Z3 proved a mathematical invariant violation.
pub const SMT_INVARIANT_VIOLATION: &str = "S011";
/// SEP-41 token interface deviation.
pub const SEP41_INTERFACE_DEVIATION: &str = "S012";
/// Reentrancy vulnerability detected (state mutation before external call without guard).
pub const REENTRANCY: &str = "S013";
/// Hardcoded secret key detected in contract source.
pub const HARDCODED_SECRET_KEY: &str = "S015";
/// Integer truncation (e.g. `as u32`) or unchecked slice/array indexing.
pub const TRUNCATION_BOUNDS: &str = "S016";

/// A single finding-code entry with machine-readable code, category, and
/// human-readable description.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct FindingCode {
    /// Short code such as `"S001"`.
    pub code: &'static str,
    /// Broad category (e.g. `"authentication"`).
    pub category: &'static str,
    /// One-line description of the finding.
    pub description: &'static str,
}

/// Returns every finding code known to this version of Sanctifier.
pub fn all_finding_codes() -> Vec<FindingCode> {
    vec![
        FindingCode {
            code: ANALYSIS_TIMEOUT,
            category: "timeout",
            description: "Analysis of a file was aborted because it exceeded the per-file timeout",
        },
        FindingCode {
            code: AUTH_GAP,
            category: "authentication",
            description: "Missing authentication guard in a privileged state-changing or external-call function",
        },
        FindingCode {
            code: PANIC_USAGE,
            category: "panic_handling",
            description: "panic!/unwrap/expect usage that may cause runtime aborts",
        },
        FindingCode {
            code: ARITHMETIC_OVERFLOW,
            category: "arithmetic",
            description: "Unchecked arithmetic operation with overflow/underflow risk",
        },
        FindingCode {
            code: LEDGER_SIZE_RISK,
            category: "storage_limits",
            description: "Ledger entry size is exceeding or approaching configured threshold",
        },
        FindingCode {
            code: STORAGE_COLLISION,
            category: "storage_keys",
            description: "Potential storage key collision across contract data paths",
        },
        FindingCode {
            code: UNSAFE_PATTERN,
            category: "unsafe_patterns",
            description: "Potentially unsafe language/runtime pattern was detected",
        },
        FindingCode {
            code: CUSTOM_RULE_MATCH,
            category: "custom_rule",
            description: "User-defined rule matched contract source",
        },
        FindingCode {
            code: EVENT_INCONSISTENCY,
            category: "events",
            description: "Inconsistent topic counts or sub-optimal gas patterns in events",
        },
        FindingCode {
            code: UNHANDLED_RESULT,
            category: "logic",
            description: "A function call returns a Result that is not consumed or handled",
        },
        FindingCode {
            code: UPGRADE_RISK,
            category: "upgrades",
            description: "Potential security risk in contract upgrade or admin mechanisms",
        },
        FindingCode {
            code: SMT_INVARIANT_VIOLATION,
            category: "formal_verification",
            description: "Formal verification (Z3) proved a mathematical violation of an invariant",
        },
        FindingCode {
            code: SEP41_INTERFACE_DEVIATION,
            category: "token_interface",
            description: "SEP-41 token interface compatibility or authorization deviation",
        },
        FindingCode {
            code: REENTRANCY,
            category: "reentrancy",
            description: "State mutation before external call without a reentrancy guard",
        },
        FindingCode {
            code: HARDCODED_SECRET_KEY,
            category: "secrets",
            description: "Hardcoded secret key detected in contract source",
        },
        FindingCode {
            code: TRUNCATION_BOUNDS,
            category: "truncation_bounds",
            description: "Integer truncation cast or unchecked array/slice indexing",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn finding_codes_are_unique() {
        let codes = all_finding_codes();
        let unique: HashSet<&str> = codes.iter().map(|c| c.code).collect();
        assert_eq!(codes.len(), unique.len());
    }

    #[test]
    fn includes_expected_codes() {
        let codes = all_finding_codes();
        assert!(codes.iter().any(|c| c.code == AUTH_GAP));
        assert!(codes.iter().any(|c| c.code == PANIC_USAGE));
        assert!(codes.iter().any(|c| c.code == ARITHMETIC_OVERFLOW));
        assert!(codes.iter().any(|c| c.code == LEDGER_SIZE_RISK));
        assert!(codes.iter().any(|c| c.code == STORAGE_COLLISION));
        assert!(codes.iter().any(|c| c.code == UNSAFE_PATTERN));
        assert!(codes.iter().any(|c| c.code == CUSTOM_RULE_MATCH));
        assert!(codes.iter().any(|c| c.code == EVENT_INCONSISTENCY));
        assert!(codes.iter().any(|c| c.code == SEP41_INTERFACE_DEVIATION));
        assert!(codes.iter().any(|c| c.code == HARDCODED_SECRET_KEY));
        assert!(codes.iter().any(|c| c.code == TRUNCATION_BOUNDS));
    }
}
