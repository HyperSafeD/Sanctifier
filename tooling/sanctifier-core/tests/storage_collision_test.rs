//! Unit tests for the S005 storage-key collision detection pass.
//!
//! # Threat model (S005)
//!
//! Soroban contracts share a flat key-value store partitioned by storage type
//! (instance / persistent / temporary).  Within one storage type any two
//! `set`/`get` calls that use the same key value silently alias each other —
//! one will overwrite the other's data without any compile- or run-time error.
//!
//! ## Attack surface
//! * **Intra-function collision** — two paths in the same function resolve to
//!   the same key.
//! * **Cross-function collision** — separate entry-points share a key, breaking
//!   logical separation (e.g. "staking balance" and "reward balance" both stored
//!   under `"BALANCE"`).
//! * **Enum-variant aliasing** — two `DataKey` variants that serialise to the
//!   same byte string collide invisibly.
//!
//! The scanner is *intra-file* and *intra-storage-type*.  Cross-file collisions
//! and cross-contract collisions are outside scope for this pass.
//!
//! ## Non-goals (by design)
//! * The same key reused across *different* storage types (instance vs
//!   persistent) is **not** a collision — storage types are independent
//!   namespaces in the Soroban host.
//! * Dynamic keys (variables, function-call results) cannot be resolved
//!   statically and are not flagged.

use sanctifier_core::{Analyzer, SanctifyConfig};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {name} not readable: {e}"))
}

// ── Fixture-based tests ──────────────────────────────────────────────────────

#[test]
fn collision_fixture_emits_at_least_one_finding() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = fixture("storage_collision_contract.rs");
    let collisions = analyzer.scan_storage_collisions(&source);
    assert!(
        !collisions.is_empty(),
        "fixture with duplicate persistent key should emit findings; got none"
    );
}

#[test]
fn collision_fixture_finding_names_the_colliding_key() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = fixture("storage_collision_contract.rs");
    let collisions = analyzer.scan_storage_collisions(&source);
    assert!(
        collisions.iter().any(|c| c.key_value.contains("BALANCE")),
        "finding should name the colliding key; got: {collisions:?}"
    );
}

// ── Inline threat-model unit tests ───────────────────────────────────────────

#[test]
fn cross_function_persistent_collision_detected() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn set_staking(env: Env) {
                env.storage().persistent().set(&"TOTAL", &100i128);
            }
            pub fn set_rewards(env: Env) {
                env.storage().persistent().set(&"TOTAL", &200i128);
            }
        }
    "#;
    let collisions = analyzer.scan_storage_collisions(source);
    assert!(
        !collisions.is_empty(),
        "cross-function persistent key collision should be detected"
    );
    assert!(collisions.iter().any(|c| c.key_value.contains("TOTAL")));
}

#[test]
fn cross_storage_type_reuse_is_not_a_collision() {
    // Same key across instance/persistent/temporary is safe — different namespaces.
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn a(env: Env) { env.storage().instance().set(&"KEY", &1u32); }
            pub fn b(env: Env) { env.storage().persistent().set(&"KEY", &2u32); }
            pub fn c(env: Env) { env.storage().temporary().set(&"KEY", &3u32); }
        }
    "#;
    let collisions = analyzer.scan_storage_collisions(source);
    assert!(
        collisions.is_empty(),
        "cross-storage-type key reuse must NOT be flagged; got: {collisions:?}"
    );
}

#[test]
fn empty_source_produces_no_collision_findings() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    assert!(analyzer.scan_storage_collisions("").is_empty());
}

#[test]
fn single_key_usage_is_not_a_collision() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn store(env: Env) {
                env.storage().persistent().set(&"UNIQUE", &42u32);
            }
        }
    "#;
    let collisions = analyzer.scan_storage_collisions(source);
    assert!(
        collisions.is_empty(),
        "single key usage must not be flagged as collision; got: {collisions:?}"
    );
}

#[test]
fn collision_finding_message_is_non_empty() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn write_a(env: Env) { env.storage().instance().set(&"SHARED", &1u32); }
            pub fn write_b(env: Env) { env.storage().instance().set(&"SHARED", &2u32); }
        }
    "#;
    let collisions = analyzer.scan_storage_collisions(source);
    for c in &collisions {
        assert!(
            !c.message.is_empty(),
            "every collision finding must carry a message"
        );
    }
}

#[test]
fn multiple_distinct_keys_in_same_storage_type_no_collision() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn write_balance(env: Env) {
                env.storage().persistent().set(&"BAL", &100i128);
            }
            pub fn write_nonce(env: Env) {
                env.storage().persistent().set(&"NONCE", &1u64);
            }
            pub fn write_admin(env: Env) {
                env.storage().persistent().set(&"ADMIN", &true);
            }
        }
    "#;
    let collisions = analyzer.scan_storage_collisions(source);
    assert!(
        collisions.is_empty(),
        "three distinct persistent keys must not produce collisions; got: {collisions:?}"
    );
}

#[test]
fn temporary_storage_collision_detected() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn cache_a(env: Env) {
                env.storage().temporary().set(&"CACHE", &1u32);
            }
            pub fn cache_b(env: Env) {
                env.storage().temporary().set(&"CACHE", &2u32);
            }
        }
    "#;
    let collisions = analyzer.scan_storage_collisions(source);
    assert!(
        !collisions.is_empty(),
        "temporary storage collision should be detected"
    );
}
