//! Standardized SEP-41 compliance suite.
//!
//! This is the single, shared conformance suite that every SEP-41 token in the
//! workspace must pass. It runs the generic harness in
//! `sanctifier_test_support::sep41_compliance` against each token's source.
//!
//! Run it with:
//!
//! ```bash
//! cargo test -p sep41-compliance
//! ```
//!
//! ## Adding a contract
//!
//! Add one `assert_compliant(...)` line in [`compliant_contracts`] pointing at
//! the contract's source. The harness verifies all ten SEP-41 functions —
//! presence, exact signature, and caller authorization — and any deviation
//! fails the test.

use sanctifier_test_support::sep41_compliance::{
    assert_compliant, assert_deviates, check, IssueKind, REQUIRED_FUNCTION_COUNT,
};

/// Every contract here is asserted to be a fully compliant SEP-41 token.
///
/// The suite intentionally covers **more than three** token contracts:
///
/// 1. `my-contract` — a real, in-repo SEP-41 token implementation.
/// 2. `amm-lp-token` — reference LP token an `amm-pool` mints for liquidity
///    shares (`contracts/amm-pool`).
/// 3. `deposit-receipt-token` — reference receipt token a `deposit-withdraw`
///    vault mints for depositor claims (`contracts/deposit-withdraw`); also
///    exercises the `require_auth_for_args` authorization variant.
fn compliant_contracts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("my-contract", include_str!("../../my-contract/src/lib.rs")),
        ("amm-lp-token", include_str!("fixtures/amm_lp_token.rs")),
        (
            "deposit-receipt-token",
            include_str!("fixtures/deposit_receipt_token.rs"),
        ),
    ]
}

#[test]
fn suite_covers_at_least_three_contracts() {
    assert!(
        compliant_contracts().len() >= 3,
        "the SEP-41 compliance suite must run against at least 3 contracts"
    );
}

#[test]
fn sep41_interface_has_ten_functions() {
    assert_eq!(REQUIRED_FUNCTION_COUNT, 10);
}

/// Every compliant contract passes the full SEP-41 conformance check.
#[test]
fn all_token_contracts_are_sep41_compliant() {
    for (name, source) in compliant_contracts() {
        assert_compliant(name, source);
    }
}

// ── Negative tests: any deviation must fail ─────────────────────────────────
//
// These prove the suite is not vacuously green — a contract that drifts from
// the SEP-41 interface is reliably caught.

#[test]
fn missing_function_fails_compliance() {
    assert_deviates(
        "noncompliant-missing-function",
        include_str!("fixtures/noncompliant_missing_function.rs"),
        IssueKind::MissingFunction,
    );
}

#[test]
fn wrong_signature_fails_compliance() {
    assert_deviates(
        "noncompliant-wrong-signature",
        include_str!("fixtures/noncompliant_wrong_signature.rs"),
        IssueKind::SignatureMismatch,
    );
}

#[test]
fn missing_authorization_fails_compliance() {
    assert_deviates(
        "noncompliant-missing-auth",
        include_str!("fixtures/noncompliant_missing_auth.rs"),
        IssueKind::AuthorizationMismatch,
    );
}

/// A non-token contract is never reported as a compliant SEP-41 token.
#[test]
fn non_token_contract_is_not_compliant() {
    let report = check("pub struct C; impl C { pub fn ping(e: Env) -> u32 { 0 } }");
    assert!(!report.compliant);
}
