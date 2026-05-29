//! Canonical finding codes emitted by Sanctifier analysis passes.
//!
//! Each constant (`S000` – `S016`) maps to a single diagnostic category.
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
/// Potential administrative centralisation or insecure override.
pub const ADMIN_TRUST_RISK: &str = "S014";
/// Hardcoded secret key or sensitive mnemonic in contract source.
pub const HARDCODED_SECRET_KEY: &str = "S015";
/// Integer truncation (cast) or unchecked slice/array indexing.
pub const TRUNCATION_BOUNDS: &str = "S016";
/// contractimport signature does not match actual implemented workspace source.
pub const CONTRACTIMPORT_MISMATCH: &str = "S017";
/// Use of PRNG without proper seeding in state-critical code.
pub const UNSAFE_PRNG: &str = "S018";
/// Unchecked return value from external Soroban cross-contract call.
pub const UNCHECKED_EXTERNAL_CALL: &str = "S019";
/// Missing event emission for privileged state changes.
pub const MISSING_STATE_EVENT: &str = "S020";
/// Per-user or large dataset stored in Instance storage instead of Persistent.
pub const INSTANCE_STORAGE_MISUSE: &str = "S021";
/// Raw `invoke_contract` call without `try_invoke_contract` error handling.
pub const RAW_INVOKE_CONTRACT: &str = "S022";
/// `#[test]` function that never references a `ContractClient`, bypassing the host-function boundary.
pub const SHALLOW_TEST: &str = "S023";
/// transfer_from-style function consumes 'from' balance without allowance check.
pub const TRANSFER_FROM_NO_ALLOWANCE: &str = "S023";

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
            code: ADMIN_TRUST_RISK,
            category: "centralization",
            description: "Excessive administrative control or insecure credential management",
        },
        FindingCode {
            code: HARDCODED_SECRET_KEY,
            category: "secrets",
            description: "Hardcoded secret key or sensitive mnemonic in contract source",
        },
        FindingCode {
            code: TRUNCATION_BOUNDS,
            category: "truncation_bounds",
            description: "Integer truncation cast or unchecked array/slice indexing",
        },
        FindingCode {
            code: CONTRACTIMPORT_MISMATCH,
            category: "integration",
            description: "contractimport signature does not match actual implemented workspace source",
        },
        FindingCode {
            code: UNSAFE_PRNG,
            category: "randomness",
            description: "Use of PRNG without proper seeding in state-critical code that could lead to predictable randomness",
        },
        FindingCode {
            code: UNCHECKED_EXTERNAL_CALL,
            category: "external_calls",
            description: "Result from cross-contract call is not checked, which may leave state inconsistent",
        },
        FindingCode {
            code: MISSING_STATE_EVENT,
            category: "events",
            description: "Privileged state change (admin, pause, upgrade) without event emission breaks off-chain data integrity",
        },
        FindingCode {
            code: INSTANCE_STORAGE_MISUSE,
            category: "storage_type",
            description: "Per-user or large dataset stored in Instance storage instead of Persistent, causing ledger entry bloat",
        },
        FindingCode {
            code: RAW_INVOKE_CONTRACT,
            category: "error_handling",
            description: "Cross-contract call via `invoke_contract` panics on callee failure; use `try_invoke_contract` with explicit Result handling",
        },
        FindingCode {
            code: SHALLOW_TEST,
            category: "test_quality",
            description: "#[test] function never references a ContractClient, bypassing serialization and auth paths exercised by the Soroban host-function boundary",
            code: TRANSFER_FROM_NO_ALLOWANCE,
            category: "token_safety",
            description: "transfer_from-style function moves 'from' balance without checking or decrementing the spender's allowance, allowing any caller to drain any account",
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
        assert!(codes.iter().any(|c| c.code == CONTRACTIMPORT_MISMATCH));
        assert!(codes.iter().any(|c| c.code == UNSAFE_PRNG));
        assert!(codes.iter().any(|c| c.code == UNCHECKED_EXTERNAL_CALL));
        assert!(codes.iter().any(|c| c.code == MISSING_STATE_EVENT));
        assert!(codes.iter().any(|c| c.code == INSTANCE_STORAGE_MISUSE));
        assert!(codes.iter().any(|c| c.code == RAW_INVOKE_CONTRACT));
        assert!(codes.iter().any(|c| c.code == SHALLOW_TEST));
        assert!(codes.iter().any(|c| c.code == TRANSFER_FROM_NO_ALLOWANCE));
    }
}
