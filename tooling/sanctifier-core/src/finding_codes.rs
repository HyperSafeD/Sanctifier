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
pub const TRANSFER_FROM_NO_ALLOWANCE: &str = "S024";
/// Persistent or Temporary storage write without a corresponding TTL bump (extend_ttl).
pub const MISSING_TTL_BUMP: &str = "S025";
/// Taint propagation finding — user-controlled data reaches a sensitive sink.
pub const TAINT_PROPAGATION: &str = "S026";
/// External call before state write without a reentrancy guard (static, complement to runtime guard).
pub const STATIC_REENTRANCY: &str = "S027";
/// Usage of storage/deployment APIs that were removed or renamed in Soroban SDK v22.
pub const DEPRECATED_SDK_USAGE: &str = "S028";
/// Use of env.ledger().timestamp() as entropy for randomness.
pub const TIMESTAMP_RANDOMNESS: &str = "S029";
/// require_auth used instead of require_auth_for_args in multi-arg admin operations, enabling replay/scope-confusion attacks.
pub const REQUIRE_AUTH_FOR_ARGS: &str = "S030";

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
    /// Short human-readable title.
    pub title: &'static str,
    /// Severity level of the finding.
    pub severity: FindingSeverity,
    /// Remediation guidance.
    pub remediation: &'static str,
    /// URL to documentation.
    pub doc_url: &'static str,
}

/// Returns every finding code known to this version of Sanctifier.
pub fn all_finding_codes() -> Vec<FindingCode> {
    vec![
        FindingCode {
            code: ANALYSIS_TIMEOUT,
            category: "timeout",
            description: "Analysis of a file was aborted because it exceeded the per-file timeout",
            title: "Analysis Timeout",
            severity: FindingSeverity::Info,
            remediation: "Increase the per-file timeout with --timeout or split the file into smaller modules",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: AUTH_GAP,
            category: "authentication",
            description: "Missing authentication guard in a privileged state-changing or external-call function",
            title: "Missing Authorization Guard",
            severity: FindingSeverity::Critical,
            remediation: "Add require_auth or require_auth_for_args before any state mutation or privileged operation",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: PANIC_USAGE,
            category: "panic_handling",
            description: "panic!/unwrap/expect usage that may cause runtime aborts",
            title: "Panic Usage",
            severity: FindingSeverity::Medium,
            remediation: "Replace panic!/unwrap/expect with proper error handling using Result and try_invoke_contract",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: ARITHMETIC_OVERFLOW,
            category: "arithmetic",
            description: "Unchecked arithmetic operation with overflow/underflow risk",
            title: "Unchecked Arithmetic",
            severity: FindingSeverity::Medium,
            remediation: "Use checked_add/checked_sub/checked_mul/checked_div/checked_rem or saturating variants to handle overflow/underflow safely",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: LEDGER_SIZE_RISK,
            category: "storage_limits",
            description: "Ledger entry size is exceeding or approaching configured threshold",
            title: "Ledger Entry Size Risk",
            severity: FindingSeverity::Medium,
            remediation: "Reduce the size of #[contracttype] structs/enums, split data across multiple keys, or raise the ledger_limit in .sanctify.toml",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: STORAGE_COLLISION,
            category: "storage_keys",
            description: "Potential storage key collision across contract data paths",
            title: "Storage Key Collision",
            severity: FindingSeverity::Medium,
            remediation: "Use unique prefixes per data domain (e.g. Symbol::new vs symbol_short!) and avoid reusing the same storage key for different data types",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: UNSAFE_PATTERN,
            category: "unsafe_patterns",
            description: "Potentially unsafe language/runtime pattern was detected",
            title: "Unsafe Pattern",
            severity: FindingSeverity::Medium,
            remediation: "Review the flagged pattern and replace with the suggested safe alternative or add explicit safety checks",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: CUSTOM_RULE_MATCH,
            category: "custom_rule",
            description: "User-defined rule matched contract source",
            title: "Custom Rule Match",
            severity: FindingSeverity::Info,
            remediation: "Review the matched pattern and address the issue according to your project's security policy",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: EVENT_INCONSISTENCY,
            category: "events",
            description: "Inconsistent topic counts or sub-optimal gas patterns in events",
            title: "Event Inconsistency",
            severity: FindingSeverity::Low,
            remediation: "Ensure all event topics follow a consistent pattern and use the optimal number of topics for gas efficiency",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: UNHANDLED_RESULT,
            category: "logic",
            description: "A function call returns a Result that is not consumed or handled",
            title: "Unhandled Result",
            severity: FindingSeverity::Medium,
            remediation: "Handle the Result with match, if let, or use the ? operator to propagate errors instead of discarding them",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: UPGRADE_RISK,
            category: "upgrades",
            description: "Potential security risk in contract upgrade or admin mechanisms",
            title: "Upgrade Risk",
            severity: FindingSeverity::Medium,
            remediation: "Implement timelock delays, multi-signature governance, and require_auth checks on all upgrade paths",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: SMT_INVARIANT_VIOLATION,
            category: "formal_verification",
            description: "Formal verification (Z3) proved a mathematical violation of an invariant",
            title: "SMT Invariant Violation",
            severity: FindingSeverity::High,
            remediation: "Fix the mathematical invariant violation identified by Z3 — review the counterexample trace and correct the contract logic",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: SEP41_INTERFACE_DEVIATION,
            category: "token_interface",
            description: "SEP-41 token interface compatibility or authorization deviation",
            title: "SEP-41 Interface Deviation",
            severity: FindingSeverity::Medium,
            remediation: "Align your token contract with the SEP-41 standard; ensure all required functions, parameters, and auth patterns match the specification",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: REENTRANCY,
            category: "reentrancy",
            description: "State mutation before external call without a reentrancy guard",
            title: "Reentrancy",
            severity: FindingSeverity::Critical,
            remediation: "Apply checks-effects-interactions pattern: move all state mutations before external calls, or add a reentrancy guard mutex",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: ADMIN_TRUST_RISK,
            category: "centralization",
            description: "Excessive administrative control or insecure credential management",
            title: "Admin Trust Risk",
            severity: FindingSeverity::Medium,
            remediation: "Decentralize admin keys, use multi-sig or timelock contracts, and rotate credentials regularly",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: HARDCODED_SECRET_KEY,
            category: "secrets",
            description: "Hardcoded secret key or sensitive mnemonic in contract source",
            title: "Hardcoded Secret Key",
            severity: FindingSeverity::Critical,
            remediation: "Remove the secret from source code immediately. Use environment variables, encrypted vaults, or secret management services instead",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: TRUNCATION_BOUNDS,
            category: "truncation_bounds",
            description: "Integer truncation cast or unchecked array/slice indexing",
            title: "Truncation / Bounds Risk",
            severity: FindingSeverity::Medium,
            remediation: "Use try_from().unwrap() or checked conversion for integer casts, and verify array/slice indices before access",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: CONTRACTIMPORT_MISMATCH,
            category: "integration",
            description: "contractimport signature does not match actual implemented workspace source",
            title: "ContractImport Mismatch",
            severity: FindingSeverity::High,
            remediation: "Ensure the #[contractimport] signature matches the actual contract implementation — update the import or fix the implementation",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: UNSAFE_PRNG,
            category: "randomness",
            description: "Use of PRNG without proper seeding in state-critical code that could lead to predictable randomness",
            title: "Unsafe PRNG Usage",
            severity: FindingSeverity::Medium,
            remediation: "Always seed env.prng() with a fresh entropy source (e.g. env.prng().seed_from_env()) before generating random values in state-critical code",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/unsafe-prng.md",
        },
        FindingCode {
            code: UNCHECKED_EXTERNAL_CALL,
            category: "external_calls",
            description: "Result from cross-contract call is not checked, which may leave state inconsistent",
            title: "Unchecked External Call",
            severity: FindingSeverity::Medium,
            remediation: "Check the return value of cross-contract calls using try_invoke_contract and handle both success and error cases explicitly",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: MISSING_STATE_EVENT,
            category: "events",
            description: "Privileged state change (admin, pause, upgrade) without event emission breaks off-chain data integrity",
            title: "Missing State Event",
            severity: FindingSeverity::Medium,
            remediation: "Emit an event for every privileged state change so off-chain indexers and monitors can track contract activity",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: INSTANCE_STORAGE_MISUSE,
            category: "storage_type",
            description: "Per-user or large dataset stored in Instance storage instead of Persistent, causing ledger entry bloat",
            title: "Instance Storage Misuse",
            severity: FindingSeverity::Medium,
            remediation: "Move per-user data to persistent() storage tier; use instance() only for contract-global configuration data",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: RAW_INVOKE_CONTRACT,
            category: "error_handling",
            description: "Cross-contract call via `invoke_contract` panics on callee failure; use `try_invoke_contract` with explicit Result handling",
            title: "Raw Invoke Contract",
            severity: FindingSeverity::Medium,
            remediation: "Replace invoke_contract with try_invoke_contract and handle the Result to gracefully recover from callee failures",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: SHALLOW_TEST,
            category: "test_quality",
            description: "#[test] function never references a ContractClient, bypassing serialization and auth paths exercised by the Soroban host-function boundary",
            title: "Shallow Test",
            severity: FindingSeverity::Low,
            remediation: "Use a ContractClient in your test to exercise the full host-function boundary, including serialization and authentication paths",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: TRANSFER_FROM_NO_ALLOWANCE,
            category: "token_safety",
            description: "transfer_from-style function moves 'from' balance without checking or decrementing the spender's allowance, allowing any caller to drain any account",
            title: "Transfer-From Without Allowance",
            severity: FindingSeverity::Critical,
            remediation: "Read and verify the allowance for (from, spender) before decrementing 'from' balance; decrement the allowance atomically with the transfer",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/transfer-from-no-allowance.md",
        },
        FindingCode {
            code: MISSING_TTL_BUMP,
            category: "storage_ttl",
            description: "Persistent or Temporary storage entry written without a corresponding extend_ttl call — entry may silently expire",
            title: "Missing TTL Bump",
            severity: FindingSeverity::Medium,
            remediation: "After writing to persistent() or temporary() storage, call extend_ttl with appropriate low and high ledger thresholds",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/missing-ttl-bump.md",
        },
        FindingCode {
            code: TAINT_PROPAGATION,
            category: "taint_analysis",
            description: "User-controlled data (tainted source) reaches a sensitive sink without sanitisation, including through tuple/struct destructures",
            title: "Taint Propagation",
            severity: FindingSeverity::High,
            remediation: "Sanitize user-controlled input before it reaches sensitive sinks; validate, bound-check, or transform the data at the taint boundary",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: STATIC_REENTRANCY,
            category: "reentrancy",
            description: "External contract call precedes a storage mutation without a reentrancy guard — classic checks-effects-interactions violation",
            title: "Static Reentrancy",
            severity: FindingSeverity::Medium,
            remediation: "Reorder operations to follow the checks-effects-interactions pattern, or add a reentrancy guard before the external call",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/static-reentrancy.md",
        },
        FindingCode {
            code: DEPRECATED_SDK_USAGE,
            category: "sdk_migration",
            description: "Usage of a storage or deployment API removed or renamed in Soroban SDK v22 — bump(), RawVal, and deployer().deploy() must be migrated",
            title: "Deprecated SDK Usage",
            severity: FindingSeverity::High,
            remediation: "Migrate bump() to extend_ttl(), replace RawVal with Val, and use new deploy patterns from the Environment",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md",
        },
        FindingCode {
            code: TIMESTAMP_RANDOMNESS,
            category: "randomness",
            description: "Block timestamp (env.ledger().timestamp()) used as entropy for randomness — validators can manipulate timestamps within bounds",
            title: "Timestamp Used as Randomness",
            severity: FindingSeverity::High,
            remediation: "Never use env.ledger().timestamp() as a sole source of randomness. Use a VRF oracle or combine multiple unpredictable entropy sources",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/unsafe-prng.md",
        },
        FindingCode {
            code: REQUIRE_AUTH_FOR_ARGS,
            category: "authentication",
            description: "Function with multiple Address parameters uses require_auth instead of require_auth_for_args, enabling replay/scope-confusion attacks on multi-arg admin operations",
            title: "Missing require_auth_for_args",
            severity: FindingSeverity::High,
            remediation: "Replace require_auth() with require_auth_for_args() to bind authorization to the exact call payload, preventing replay attacks across different argument combinations",
            doc_url: "https://github.com/HyperSafeD/Sanctifier/blob/main/docs/rules/require-auth-for-args.md",
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
        assert!(codes.iter().any(|c| c.code == MISSING_TTL_BUMP));
        assert!(codes.iter().any(|c| c.code == TAINT_PROPAGATION));
        assert!(codes.iter().any(|c| c.code == STATIC_REENTRANCY));
        assert!(codes.iter().any(|c| c.code == DEPRECATED_SDK_USAGE));
        assert!(codes.iter().any(|c| c.code == TIMESTAMP_RANDOMNESS));
        assert!(codes.iter().any(|c| c.code == REQUIRE_AUTH_FOR_ARGS));
    }
}
