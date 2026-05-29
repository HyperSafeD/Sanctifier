//! Integration / e2e tests for `AnalysisCache` — cache design & invalidation (#513).
//!
//! These tests drive the cache through the real `Analyzer` API to verify that:
//! 1. A cache hit returns a result identical to a fresh analysis.
//! 2. Source changes correctly invalidate the cached entry.
//! 3. Explicit invalidation forces a recompute.
//! 4. The cache stays within its capacity bound under workspace-scale loads.
//! 5. CI behaviour is deterministic across repeated runs.

use sanctifier_core::analysis_cache::AnalysisCache;
use sanctifier_core::{Analyzer, SanctifyConfig};

// ── Fixtures ──────────────────────────────────────────────────────────────────

const CLEAN_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env};
    #[contract] pub struct Token;
    #[contractimpl] impl Token {
        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            from.require_auth();
        }
    }
"#;

const OVERFLOW_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Env};
    #[contract] pub struct Overflow;
    #[contractimpl] impl Overflow {
        pub fn add(_env: Env, a: u32, b: u32) -> u32 { a + b }
    }
"#;

const AUTH_GAP_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
    #[contract] pub struct AuthGap;
    #[contractimpl] impl AuthGap {
        pub fn store_user(env: Env, user: Address) {
            env.storage().instance().set(&Symbol::new(&env, "user"), &user);
        }
    }
"#;

fn analyzer() -> Analyzer {
    Analyzer::new(SanctifyConfig::default())
}

// ── Cache hit returns identical findings ──────────────────────────────────────

#[test]
fn cache_hit_arithmetic_findings_match_fresh_analysis() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::ArithmeticIssue>> = AnalysisCache::new(16);

    let fresh = a.scan_arithmetic_overflow(OVERFLOW_CONTRACT);
    let cached = cache.get_or_analyze("overflow.rs", OVERFLOW_CONTRACT, || {
        a.scan_arithmetic_overflow(OVERFLOW_CONTRACT)
    });
    let cached2 = cache.get_or_analyze("overflow.rs", OVERFLOW_CONTRACT, || {
        a.scan_arithmetic_overflow(OVERFLOW_CONTRACT)
    });

    assert_eq!(
        fresh.len(),
        cached.len(),
        "cache hit count must match fresh count"
    );
    assert_eq!(
        cached.len(),
        cached2.len(),
        "repeated cache hits must be stable"
    );
}

#[test]
fn cache_hit_auth_gap_findings_match_fresh_analysis() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::AuthGapIssue>> = AnalysisCache::new(16);

    let fresh = a.scan_auth_gaps(AUTH_GAP_CONTRACT);
    let cached = cache.get_or_analyze("auth_gap.rs", AUTH_GAP_CONTRACT, || {
        a.scan_auth_gaps(AUTH_GAP_CONTRACT)
    });

    assert_eq!(fresh.len(), cached.len());
    for (f, c) in fresh.iter().zip(cached.iter()) {
        assert_eq!(f.function_name, c.function_name);
    }
}

#[test]
fn clean_contract_produces_zero_findings_through_cache() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::ArithmeticIssue>> = AnalysisCache::new(16);

    for _ in 0..5 {
        let result = cache.get_or_analyze("clean.rs", CLEAN_CONTRACT, || {
            a.scan_arithmetic_overflow(CLEAN_CONTRACT)
        });
        assert!(
            result.is_empty(),
            "clean contract must produce no arithmetic findings"
        );
    }
}

// ── Cache invalidation on source change ───────────────────────────────────────

#[test]
fn source_change_invalidates_cache_and_returns_new_findings() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::ArithmeticIssue>> = AnalysisCache::new(16);

    let before = cache.get_or_analyze("contract.rs", CLEAN_CONTRACT, || {
        a.scan_arithmetic_overflow(CLEAN_CONTRACT)
    });
    assert!(before.is_empty());

    // Same key, different source — must invalidate and recompute.
    let after = cache.get_or_analyze("contract.rs", OVERFLOW_CONTRACT, || {
        a.scan_arithmetic_overflow(OVERFLOW_CONTRACT)
    });
    assert!(
        !after.is_empty(),
        "overflow findings must appear after source change"
    );
}

#[test]
fn explicit_invalidate_forces_recompute() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::ArithmeticIssue>> = AnalysisCache::new(16);

    cache.get_or_analyze("c.rs", CLEAN_CONTRACT, || {
        a.scan_arithmetic_overflow(CLEAN_CONTRACT)
    });
    assert!(cache.is_cached("c.rs", CLEAN_CONTRACT));

    cache.invalidate("c.rs");
    assert!(!cache.is_cached("c.rs", CLEAN_CONTRACT));

    // After invalidation the value is recomputed correctly.
    let recomputed = cache.get_or_analyze("c.rs", CLEAN_CONTRACT, || {
        a.scan_arithmetic_overflow(CLEAN_CONTRACT)
    });
    assert!(recomputed.is_empty());
}

// ── Workspace-scale: multiple files ───────────────────────────────────────────

#[test]
fn cache_handles_multiple_files_independently() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::ArithmeticIssue>> = AnalysisCache::new(32);

    let files = [
        ("clean.rs", CLEAN_CONTRACT),
        ("overflow.rs", OVERFLOW_CONTRACT),
    ];

    // First pass — populate.
    for (name, src) in &files {
        cache.get_or_analyze(name, src, || a.scan_arithmetic_overflow(src));
    }
    assert_eq!(cache.len(), 2);

    // Second pass — all hits, no recompute.
    for (name, src) in &files {
        assert!(cache.is_cached(name, src), "{name} should be cached");
    }
}

#[test]
fn cache_stays_within_capacity_across_workspace_scan() {
    let a = analyzer();
    let capacity = 5usize;
    let mut cache: AnalysisCache<Vec<sanctifier_core::ArithmeticIssue>> =
        AnalysisCache::new(capacity);

    // Simulate scanning 20 unique files.
    for i in 0..20usize {
        let key = format!("contract_{i}.rs");
        let src = format!("fn f{i}() {{ let x = {i}u32 + 1; }}");
        cache.get_or_analyze(&key, &src, || a.scan_arithmetic_overflow(&src));
    }

    assert!(
        cache.len() <= capacity,
        "cache len {} must not exceed capacity {capacity}",
        cache.len()
    );
}

// ── CI determinism ────────────────────────────────────────────────────────────

#[test]
fn repeated_cache_reads_are_deterministic_across_invocations() {
    let a = analyzer();
    let mut cache: AnalysisCache<Vec<sanctifier_core::AuthGapIssue>> = AnalysisCache::new(16);

    let results: Vec<_> = (0..10)
        .map(|_| {
            cache.get_or_analyze("auth_gap.rs", AUTH_GAP_CONTRACT, || {
                a.scan_auth_gaps(AUTH_GAP_CONTRACT)
            })
        })
        .collect();

    let first = &results[0];
    for r in &results[1..] {
        assert_eq!(
            first.len(),
            r.len(),
            "repeated cache reads must return identical finding counts"
        );
    }
}
