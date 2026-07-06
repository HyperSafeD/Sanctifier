//! Property-based tests for the analysis engine.
//!
//! # The property
//!
//! > The analysis engine must never panic — on **any** input, valid Rust or
//! > not. For every input string, running the full rule set returns a result
//! > that is either `Ok(findings)` or `Err(parse_error)`; it never aborts the
//! > process, never overflows the stack, and never runs out of memory.
//!
//! This is the contract real users depend on: Sanctifier is run over arbitrary,
//! often half-written or machine-generated contract source, and a single panic
//! in a rule would take down an editor's LSP session or a CI run.
//!
//! # What this generates
//!
//! Three input distributions, mixed together:
//!
//! 1. **Structured Soroban source** — randomly assembled `#[contractimpl]`
//!    blocks whose bodies are built from fragments the rules specifically look
//!    for (`require_auth`, raw arithmetic, `unwrap`, storage writes, casts,
//!    `panic!`, …). These mostly parse, so they exercise rule *logic*.
//! 2. **Random Rust-ish snippets** — free-form statements and items that may or
//!    may not parse.
//! 3. **Arbitrary text** — fully random Unicode (including control characters
//!    and embedded NULs), to fuzz the parser/validator boundary.
//!
//! # Crash regressions
//!
//! Any input proptest discovers that makes the engine panic is persisted by
//! proptest to `tests/proptest-regressions/` and is replayed on every
//! subsequent run. A curated seed corpus of historically tricky inputs lives in
//! [`regression_corpus`] and is also asserted panic-free.
//!
//! # Tuning
//!
//! The case count defaults to 10,000 and can be overridden with the
//! `PROPTEST_CASES` environment variable (used by the dedicated, time-boxed CI
//! job).

use std::panic::{catch_unwind, AssertUnwindSafe};

use proptest::prelude::*;

use sanctifier_core::parser::{self, ParseError};
use sanctifier_core::rules::RuleRegistry;
use sanctifier_core::{Analyzer, SanctifyConfig};

/// Default number of generated cases. Overridable via `PROPTEST_CASES`.
fn case_count() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000)
}

/// Run the **whole** analysis engine over `source`.
///
/// Returns:
/// - `Ok(finding_count)` when the source parses and the rules run.
/// - `Err(parse_error)` when the validator or parser rejects the input.
///
/// It must do one or the other for every possible input — never panic. The
/// rule registry (`run_all`) is intentionally invoked *without* a panic guard
/// so that a panicking rule surfaces here as a test failure rather than being
/// silently swallowed.
fn run_engine(source: &str) -> Result<usize, ParseError> {
    // Validate and parse FIRST. This is both the contract (Ok/Err split) and a
    // safety gate: `parse_source` runs the input-validation guards (size, NUL
    // bytes, and delimiter-nesting depth) before any recursive rule walks the
    // source, so pathologically deep input is rejected here instead of
    // overflowing the parser/analyzer stack.
    parser::parse_source(source)?;

    let registry = RuleRegistry::with_default_rules();
    let analyzer = Analyzer::new(SanctifyConfig::default());

    // `run_all` runs the full built-in rule registry (every S0xx syntactic
    // rule). `verify_sep41_interface` covers the SEP-41 analysis that lives
    // outside the registry, so the property exercises the whole rule surface.
    let mut findings = registry.run_all(source).len();
    findings += analyzer.verify_sep41_interface(source).issues.len();

    Ok(findings)
}

/// Assert that analyzing `source` does not panic, returning a short label of
/// which arm of the contract was taken (for diagnostics on shrink).
fn assert_no_panic(source: &str) -> Result<&'static str, String> {
    let result = catch_unwind(AssertUnwindSafe(|| run_engine(source)));
    match result {
        Ok(Ok(_)) => Ok("ok(findings)"),
        Ok(Err(_)) => Ok("err(parse_error)"),
        Err(_) => Err(format!(
            "analysis engine PANICKED on input ({} bytes): {:?}",
            source.len(),
            truncate(source, 400),
        )),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

// ── Generators ──────────────────────────────────────────────────────────────

/// A short lowercase identifier.
fn arb_ident() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z][a-z0-9_]{0,7}").expect("valid regex")
}

/// A Soroban-flavored type name.
fn arb_type() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("i128"),
        Just("u32"),
        Just("i64"),
        Just("u64"),
        Just("u128"),
        Just("bool"),
        Just("Address"),
        Just("MuxedAddress"),
        Just("String"),
        Just("Bytes"),
        Just("Env"),
    ]
}

/// A single statement, drawn from fragments that the rules actively scan for.
fn arb_stmt() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("from.require_auth();".to_string()),
        Just("spender.require_auth_for_args(().into_val(&env));".to_string()),
        Just("let s = a + b;".to_string()),
        Just("let d = a - b;".to_string()),
        Just("let m = a * b;".to_string()),
        Just("let q = a / b;".to_string()),
        Just("env.storage().persistent().set(&key, &value);".to_string()),
        Just("env.storage().instance().set(&key, &value);".to_string()),
        Just("let t = env.ledger().timestamp();".to_string()),
        Just("let narrowed = wide as u32;".to_string()),
        Just("result.unwrap();".to_string()),
        Just("maybe.expect(\"must exist\");".to_string()),
        Just("panic!(\"boom\");".to_string()),
        Just("client.invoke_contract(&cid, &sym, args);".to_string()),
        Just("env.events().publish((topic,), data);".to_string()),
        Just("for i in 0..n { total += i; }".to_string()),
        arb_ident().prop_map(|i| format!("let {i} = 0i128;")),
        (arb_ident(), arb_type()).prop_map(|(i, t)| format!("let {i}: {t} = Default::default();")),
    ]
}

/// A public method with random params and a random body.
fn arb_fn() -> impl Strategy<Value = String> {
    (
        arb_ident(),
        prop::collection::vec((arb_ident(), arb_type()), 0..4),
        prop::collection::vec(arb_stmt(), 0..5),
        prop::option::of(arb_type()),
    )
        .prop_map(|(name, params, body, ret)| {
            let params: Vec<String> = std::iter::once("env: Env".to_string())
                .chain(params.into_iter().map(|(n, t)| format!("{n}: {t}")))
                .collect();
            let ret = ret.map(|t| format!(" -> {t}")).unwrap_or_default();
            let body = body.join("\n        ");
            format!("    pub fn {name}({}){ret} {{\n        {body}\n    }}", params.join(", "))
        })
}

/// A complete, mostly-parseable Soroban contract.
fn arb_contract() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_fn(), 1..4).prop_map(|fns| {
        format!(
            "#![no_std]\nuse soroban_sdk::{{contract, contractimpl, Address, Env, String}};\n\
             #[contract]\npub struct C;\n#[contractimpl]\nimpl C {{\n{}\n}}\n",
            fns.join("\n")
        )
    })
}

/// Free-form Rust-ish snippets (often invalid).
fn arb_snippet() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_stmt(), 0..12).prop_map(|stmts| stmts.join("\n"))
}

/// Fully arbitrary text, including control characters and embedded NULs.
fn arb_garbage() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..600).prop_map(|cs| cs.into_iter().collect())
}

/// The full mixed input distribution.
fn arb_source() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => arb_contract(),
        3 => arb_snippet(),
        2 => arb_garbage(),
        1 => any::<Vec<u8>>().prop_map(|b| String::from_utf8_lossy(&b).into_owned()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: case_count(),
        max_shrink_iters: 2_000,
        .. ProptestConfig::default()
    })]

    /// The engine never panics on any generated input. This is the headline
    /// property; it runs the full `PROPTEST_CASES` budget (10,000 by default).
    #[test]
    fn engine_never_panics(source in arb_source()) {
        match assert_no_panic(&source) {
            Ok(_) => {},
            Err(message) => prop_assert!(false, "{message}"),
        }
    }
}

proptest! {
    // The Ok/Err invariant is cheaper to establish and does not need the full
    // budget, so cap it to keep the suite inside the CI time box.
    #![proptest_config(ProptestConfig {
        cases: case_count().min(1_000),
        .. ProptestConfig::default()
    })]

    /// Whenever the source parses cleanly, analysis yields `Ok(findings)`;
    /// whenever it does not, analysis yields `Err(parse_error)`. The two are
    /// mutually exclusive and exhaustive.
    #[test]
    fn ok_xor_parse_error(source in arb_source()) {
        let parses = parser::parse_source(&source).is_ok();
        let outcome = catch_unwind(AssertUnwindSafe(|| run_engine(&source)));
        prop_assert!(outcome.is_ok(), "engine panicked");
        let outcome = outcome.unwrap();
        prop_assert_eq!(parses, outcome.is_ok());
    }
}

// ── Regression corpus ────────────────────────────────────────────────────────
//
// Curated inputs that have historically stressed the parser or rules. Any crash
// input discovered by proptest should be distilled to its essence and added
// here (in addition to proptest's own `tests/proptest-regressions/` artifacts),
// so the fix is locked in by a fast, deterministic test.

/// Inputs known to probe edge cases: empty, whitespace, control characters,
/// deeply nested delimiters, oversized tokens, unicode identifiers, and partial
/// Soroban constructs.
const REGRESSION_CORPUS: &[&str] = &[
    "",
    " ",
    "\n\t\r",
    "\0",
    "fn",
    "}}}}}}}}}}",
    "{{{{{{{{{{",
    "impl",
    "#[contractimpl]",
    "#[contractimpl] impl C {}",
    "pub fn transfer(env: Env, from: Address) { from.require_auth(); }",
    "impl C { pub fn f(env: Env) -> i128 { a - b } }",
    "let x = 9999999999999999999999999999999999999999;",
    "fn f() { let y = i128::MAX as u8; }",
    "//\u{0}comment with nul",
    "🦀 contract 🦀",
    "impl C { pub fn f() { f().f().f().f().f().f(); } }",
];

#[test]
fn regression_corpus_never_panics() {
    for (i, input) in REGRESSION_CORPUS.iter().enumerate() {
        assert_no_panic(input)
            .unwrap_or_else(|message| panic!("regression #{i} reintroduced a crash: {message}"));
    }

    // DISCOVERED BY PROPTEST: a deeply nested expression used to overflow the
    // recursive parser/analyzer stack and abort the process (SIGABRT). It is now
    // rejected by the `EXCESSIVE_NESTING` validation guard, so analysis returns
    // a graceful `Err(parse_error)` instead of crashing.
    let nested = format!("fn f() {{ {} }}", "(".repeat(500));
    assert_eq!(
        assert_no_panic(&nested),
        Ok("err(parse_error)"),
        "deep nesting must be rejected gracefully, not crash the process"
    );

    // A long but shallow token stream (many independent statements) must be
    // processed without crashing.
    let long_body = "let v = 1i128;\n".repeat(3_000);
    let long = format!("fn f() {{ {long_body} }}");
    assert_no_panic(&long).expect("long flat input must be handled, not crash");
}

/// Sanity check that the harness actually distinguishes the two contract arms.
#[test]
fn contract_arms_are_reachable() {
    assert_eq!(
        assert_no_panic("pub fn f(env: Env) -> i128 { 0 }"),
        Ok("ok(findings)")
    );
    assert_eq!(assert_no_panic("this is not rust {{{"), Ok("err(parse_error)"));
}
