//! Unit tests + fixtures for concurrency safety and analysis determinism (#512).
//!
//! # What is tested
//!
//! ## Determinism
//! Running any analysis pass twice on the same source must return identical
//! results.  This covers the risk of HashMap / HashSet iteration order
//! influencing finding order.
//!
//! ## Concurrency safety
//! `Analyzer` is `Send + Sync` (its fields are immutable after construction).
//! These tests spawn multiple threads that call analysis passes simultaneously
//! and assert that no panics occur and results are consistent.

use sanctifier_core::{Analyzer, SanctifyConfig};
use std::sync::Arc;
use std::thread;

// ── Fixtures ──────────────────────────────────────────────────────────────────

const OVERFLOW_SRC: &str = r#"
    use soroban_sdk::{contract, contractimpl, Env};
    #[contract] pub struct C;
    #[contractimpl] impl C {
        pub fn add(_env: Env, a: u32, b: u32) -> u32 { a + b }
        pub fn sub(_env: Env, a: u32, b: u32) -> u32 { a - b }
        pub fn mul(_env: Env, a: u32, b: u32) -> u32 { a * b }
    }
"#;

const AUTH_GAP_SRC: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
    #[contract] pub struct C;
    #[contractimpl] impl C {
        pub fn store(env: Env, addr: Address) {
            env.storage().instance().set(&Symbol::new(&env, "k"), &addr);
        }
        pub fn protected(env: Env, addr: Address) {
            addr.require_auth();
            env.storage().instance().set(&Symbol::new(&env, "k"), &addr);
        }
    }
"#;

const PANIC_SRC: &str = r#"
    use soroban_sdk::{contract, contractimpl, Env};
    #[contract] pub struct C;
    #[contractimpl] impl C {
        pub fn risky(_env: Env, v: Option<u32>) -> u32 { v.unwrap() }
        pub fn crash(_env: Env)                         { panic!("oh no"); }
    }
"#;

const CLEAN_SRC: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env};
    #[contract] pub struct Token;
    #[contractimpl] impl Token {
        pub fn transfer(_env: Env, from: Address, _to: Address, _amount: i128) {
            from.require_auth();
        }
    }
"#;

fn make_analyzer() -> Analyzer {
    Analyzer::new(SanctifyConfig::default())
}

// ── Determinism: same source → same findings ──────────────────────────────────

#[test]
fn arithmetic_findings_are_deterministic() {
    let a = make_analyzer();
    let mut runs: Vec<Vec<_>> = (0..10)
        .map(|_| {
            let mut v = a.scan_arithmetic_overflow(OVERFLOW_SRC);
            v.sort_by(|x, y| x.location.cmp(&y.location));
            v
        })
        .collect();

    let reference = runs.remove(0);
    for run in runs {
        assert_eq!(
            reference.len(),
            run.len(),
            "arithmetic finding count must be stable across runs"
        );
        for (r, s) in reference.iter().zip(run.iter()) {
            assert_eq!(r.location, s.location);
            assert_eq!(r.operation, s.operation);
        }
    }
}

#[test]
fn auth_gap_findings_are_deterministic() {
    let a = make_analyzer();
    let reference: Vec<_> = {
        let mut v = a.scan_auth_gaps(AUTH_GAP_SRC);
        v.sort_by(|x, y| x.function_name.cmp(&y.function_name));
        v
    };

    for _ in 0..10 {
        let mut v = a.scan_auth_gaps(AUTH_GAP_SRC);
        v.sort_by(|x, y| x.function_name.cmp(&y.function_name));
        assert_eq!(reference.len(), v.len());
        for (r, s) in reference.iter().zip(v.iter()) {
            assert_eq!(r.function_name, s.function_name);
        }
    }
}

#[test]
fn panic_findings_are_deterministic() {
    let a = make_analyzer();
    let runs: Vec<Vec<_>> = (0..10)
        .map(|_| {
            let mut v = a.scan_panics(PANIC_SRC);
            v.sort_by(|x, y| x.location.cmp(&y.location));
            v
        })
        .collect();

    let first_len = runs[0].len();
    for run in &runs {
        assert_eq!(
            first_len,
            run.len(),
            "panic finding count must be stable across runs"
        );
    }
}

#[test]
fn clean_source_always_produces_zero_findings() {
    let a = make_analyzer();
    for _ in 0..20 {
        assert!(a.scan_auth_gaps(CLEAN_SRC).is_empty());
        assert!(a.scan_panics(CLEAN_SRC).is_empty());
        assert!(a.scan_arithmetic_overflow(CLEAN_SRC).is_empty());
    }
}

#[test]
fn ledger_size_analysis_is_deterministic() {
    let a = make_analyzer();
    let runs: Vec<_> = (0..10)
        .map(|_| a.analyze_ledger_size(CLEAN_SRC).len())
        .collect();
    assert!(
        runs.windows(2).all(|w| w[0] == w[1]),
        "ledger size warning count must be stable"
    );
}

// ── Determinism: different sources → different findings ───────────────────────

#[test]
fn different_sources_produce_different_finding_counts() {
    let a = make_analyzer();
    let clean = a.scan_arithmetic_overflow(CLEAN_SRC).len();
    let overflow = a.scan_arithmetic_overflow(OVERFLOW_SRC).len();
    assert!(
        overflow > clean,
        "overflow contract ({overflow}) must have more findings than clean ({clean})"
    );
}

// ── Concurrency safety ────────────────────────────────────────────────────────

/// Verify `Analyzer: Send + Sync` at the type level.
#[test]
fn analyzer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Analyzer>();
}

#[test]
fn concurrent_arithmetic_scans_produce_consistent_results() {
    let analyzer = Arc::new(make_analyzer());
    let expected_len = analyzer.scan_arithmetic_overflow(OVERFLOW_SRC).len();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let a = Arc::clone(&analyzer);
            thread::spawn(move || a.scan_arithmetic_overflow(OVERFLOW_SRC).len())
        })
        .collect();

    for h in handles {
        let len = h.join().expect("thread must not panic");
        assert_eq!(
            expected_len, len,
            "concurrent arithmetic scans must return consistent finding counts"
        );
    }
}

#[test]
fn concurrent_auth_gap_scans_produce_consistent_results() {
    let analyzer = Arc::new(make_analyzer());
    let expected_len = analyzer.scan_auth_gaps(AUTH_GAP_SRC).len();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let a = Arc::clone(&analyzer);
            thread::spawn(move || a.scan_auth_gaps(AUTH_GAP_SRC).len())
        })
        .collect();

    for h in handles {
        let len = h.join().expect("thread must not panic");
        assert_eq!(expected_len, len);
    }
}

#[test]
fn concurrent_mixed_passes_do_not_panic() {
    let analyzer = Arc::new(make_analyzer());

    let handles: Vec<_> = (0..12)
        .map(|i| {
            let a = Arc::clone(&analyzer);
            thread::spawn(move || match i % 3 {
                0 => {
                    a.scan_arithmetic_overflow(OVERFLOW_SRC);
                }
                1 => {
                    a.scan_auth_gaps(AUTH_GAP_SRC);
                }
                _ => {
                    a.scan_panics(PANIC_SRC);
                }
            })
        })
        .collect();

    for h in handles {
        h.join()
            .expect("no thread should panic during concurrent analysis");
    }
}

#[test]
fn concurrent_scans_with_different_configs_do_not_interfere() {
    let strict = Arc::new(Analyzer::new(SanctifyConfig {
        strict_mode: true,
        ..Default::default()
    }));
    let normal = Arc::new(Analyzer::new(SanctifyConfig::default()));

    let handles: Vec<_> = (0..8)
        .map(|i| {
            let a = if i % 2 == 0 {
                Arc::clone(&strict)
            } else {
                Arc::clone(&normal)
            };
            thread::spawn(move || a.scan_arithmetic_overflow(OVERFLOW_SRC).len())
        })
        .collect();

    // Both configs must run without panicking; we don't assert equal counts
    // because strict mode may emit more warnings.
    for h in handles {
        h.join().expect("thread must not panic");
    }
}
