//! # `sep41-compliance` — the standard SEP-41 conformance suite
//!
//! Every SEP-41 token contract in this workspace must pass one shared,
//! standardized compliance suite rather than relying on per-contract, ad-hoc
//! tests.  This crate is that suite's home.
//!
//! - The reusable, generic harness lives in
//!   [`sanctifier_test_support::sep41_compliance`].  It takes a contract's
//!   source and verifies all ten SEP-41 functions: presence, exact signature,
//!   and caller authorization.
//! - The actual conformance tests live in `tests/compliance.rs` and run the
//!   harness against several real and reference token contracts.
//!
//! Run the whole suite with:
//!
//! ```bash
//! cargo test -p sep41-compliance
//! ```
//!
//! Adding a new token contract to the suite is one line in
//! `tests/compliance.rs`:
//!
//! ```rust,ignore
//! assert_compliant("my-new-token", include_str!("../../my-new-token/src/lib.rs"));
//! ```
//!
//! Any deviation from the SEP-41 interface — a missing function, a wrong
//! parameter type, or a missing `require_auth` — fails the suite.

/// Re-export of the conformance harness so downstream crates can depend on a
/// single, stable path (`sep41_compliance::assert_compliant`).
pub use sanctifier_test_support::sep41_compliance;
