//! Soroban v21 (Protocol 21) preview support.
//!
//! Protocol 21 was activated on Stellar Mainnet on June 18, 2024.  This module
//! documents the new host functions and storage types introduced in v21 and
//! provides helpers used by the analysis engine to recognise them.
//!
//! ## New in Soroban v21
//!
//! ### Host functions
//! - `invoke_contract_check` — like `invoke_contract` but returns a `Result`
//!   instead of panicking on failure.
//! - `prng_u64_in_inclusive_range` / `prng_bytes_new` / `prng_vec_shuffle` —
//!   pseudo-random number generator host functions.
//! - `string_copy_to_slice` / `bytes_copy_to_slice` — byte/string conversion
//!   host functions.
//!
//! ### Storage
//! - `extend_ttl(min_ledgers_to_live, max_ledgers_to_live)` — extend the
//!   time-to-live of a storage entry.  Available on `persistent()`, `temporary()`
//!   and `instance()` storage.
//!
//! ### Types
//! - `MuxedAddress` — a multiplexed Stellar account address used in the SEP-41
//!   `transfer` function signature.
//! - `Prng` — the pseudo-random number generator type.

/// All host-function names introduced or promoted in Soroban v21.
pub const V21_HOST_FUNCTIONS: &[&str] = &[
    "invoke_contract_check",
    "prng_u64_in_inclusive_range",
    "prng_bytes_new",
    "prng_vec_shuffle",
    "string_copy_to_slice",
    "bytes_copy_to_slice",
];

/// Storage methods that modify state, including the v21 `extend_ttl`.
pub const STORAGE_MUTATION_METHODS: &[&str] = &[
    "set",
    "update",
    "remove",
    "extend_ttl", // v21
];

/// External-call methods that can trigger reentrancy.
pub const EXTERNAL_CALL_METHODS: &[&str] = &[
    "invoke_contract",
    "invoke_contract_check", // v21
];

/// New Soroban v21 type names that the parser should recognise.
pub const V21_TYPE_NAMES: &[&str] = &[
    "MuxedAddress",
    "Prng",
];

/// Returns `true` if `method` is a storage-mutating operation (including v21).
pub fn is_storage_mutation(method: &str) -> bool {
    STORAGE_MUTATION_METHODS.contains(&method)
}

/// Returns `true` if `method` is an external contract call (including v21).
pub fn is_external_call(method: &str) -> bool {
    EXTERNAL_CALL_METHODS.contains(&method)
}

/// Returns `true` if `method` is a v21 PRNG host function.
pub fn is_prng_function(method: &str) -> bool {
    matches!(
        method,
        "prng_u64_in_inclusive_range" | "prng_bytes_new" | "prng_vec_shuffle"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_ttl_is_storage_mutation() {
        assert!(is_storage_mutation("extend_ttl"));
        assert!(is_storage_mutation("set"));
        assert!(is_storage_mutation("remove"));
        assert!(!is_storage_mutation("get"));
    }

    #[test]
    fn invoke_contract_check_is_external_call() {
        assert!(is_external_call("invoke_contract_check"));
        assert!(is_external_call("invoke_contract"));
        assert!(!is_external_call("get"));
    }

    #[test]
    fn v21_host_functions_are_listed() {
        assert!(V21_HOST_FUNCTIONS.contains(&"invoke_contract_check"));
        assert!(V21_HOST_FUNCTIONS.contains(&"prng_u64_in_inclusive_range"));
    }

    #[test]
    fn v21_types_are_listed() {
        assert!(V21_TYPE_NAMES.contains(&"MuxedAddress"));
        assert!(V21_TYPE_NAMES.contains(&"Prng"));
    }
}
