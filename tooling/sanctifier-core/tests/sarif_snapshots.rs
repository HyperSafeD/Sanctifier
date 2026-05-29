//! Snapshot tests for SARIF output of every built-in rule.
//!
//! Each test runs a rule against a minimal Soroban source snippet that is
//! guaranteed to trigger at least one violation and snapshots the
//! JSON-serialised `RuleViolation` list.  This catches silent regressions
//! in rule messages, severity levels, or suggestion text.
//!
//! Run locally:
//!   cargo test --test sarif_snapshots
//!
//! To regenerate snapshots after an intentional change:
//!   INSTA_UPDATE=new cargo test --test sarif_snapshots
//!   cargo insta review

use insta::with_settings;
use sanctifier_core::rules::{
    arithmetic_overflow::ArithmeticOverflowRule,
    auth_gap::AuthGapRule,
    instance_storage_misuse::InstanceStorageMisuseRule,
    ledger_size::LedgerSizeRule,
    missing_state_event::MissingStateEventRule,
    panic_detection::PanicDetectionRule,
    reentrancy::ReentrancyRule,
    shadow_storage::ShadowStorageRule,
    storage_update_state_check::StorageUpdateStateCheckRule,
    truncation_bounds::TruncationBoundsRule,
    unchecked_external_call::UncheckedExternalCallRule,
    unhandled_result::UnhandledResultRule,
    unsafe_prng::UnsafePrngRule,
    unused_variable::UnusedVariableRule,
    variable_shadowing::VariableShadowingRule,
    Rule, RuleViolation,
};

// ── Helper ────────────────────────────────────────────────────────────────────

/// Serialize violations to a JSON value, replacing the `location` field with
/// a stable placeholder so snapshots are not fragile to span/line-number shifts
/// that syn reports for string inputs.
fn violations_json(violations: &[RuleViolation]) -> serde_json::Value {
    let mut v: serde_json::Value = serde_json::to_value(violations).unwrap();
    if let Some(arr) = v.as_array_mut() {
        for item in arr.iter_mut() {
            if let Some(loc) = item.get_mut("location") {
                // Keep the function-name prefix, strip the `:line` suffix.
                let s = loc.as_str().unwrap_or("").to_string();
                let stable = s.split(':').next().unwrap_or(&s).to_string();
                *loc = serde_json::Value::String(stable);
            }
        }
    }
    v
}

// ── auth_gap ──────────────────────────────────────────────────────────────────

#[test]
fn sarif_auth_gap() {
    let source = r#"
        impl MyContract {
            pub fn withdraw(env: Env, recipient: Address, amount: i128) {
                env.storage().persistent().set(&recipient, &amount);
            }
        }
    "#;
    let rule = AuthGapRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "auth_gap must fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("auth_gap", violations_json(&violations));
    });
}

// ── panic_detection ───────────────────────────────────────────────────────────

#[test]
fn sarif_panic_detection() {
    let source = r#"
        impl MyContract {
            pub fn fund(env: Env, key: i64) {
                let _v = env.storage().persistent().get(&key).unwrap();
            }
        }
    "#;
    let rule = PanicDetectionRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "panic_detection must fire for unwrap");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("panic_detection", violations_json(&violations));
    });
}

// ── arithmetic_overflow ───────────────────────────────────────────────────────

#[test]
fn sarif_arithmetic_overflow() {
    let source = r#"
        impl MyContract {
            pub fn add(env: Env, a: i128, b: i128) -> i128 {
                a + b
            }
        }
    "#;
    let rule = ArithmeticOverflowRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "arithmetic_overflow must fire for bare +");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("arithmetic_overflow", violations_json(&violations));
    });
}

// ── unsafe_prng ───────────────────────────────────────────────────────────────

#[test]
fn sarif_unsafe_prng() {
    let source = r#"
        impl MyContract {
            pub fn draw_winner(env: Env, slot: i64) {
                let n = env.prng().u64_in_range(0..100);
                env.storage().persistent().set(&slot, &n);
            }
        }
    "#;
    let rule = UnsafePrngRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "unsafe_prng must fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("unsafe_prng", violations_json(&violations));
    });
}

// ── unhandled_result ──────────────────────────────────────────────────────────

#[test]
fn sarif_unhandled_result() {
    let source = r#"
        impl MyContract {
            pub fn transfer(env: Env, token: Address, to: Address, amount: i128) {
                token::Client::new(&env, &token).try_transfer(&env.current_contract_address(), &to, &amount);
            }
        }
    "#;
    let rule = UnhandledResultRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "unhandled_result must fire for ignored try_*");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("unhandled_result", violations_json(&violations));
    });
}

// ── shadow_storage ────────────────────────────────────────────────────────────

#[test]
fn sarif_shadow_storage() {
    let source = r#"
        impl MyContract {
            pub fn set_user_balance(env: Env, user: Address, balance: i128) {
                env.storage().instance().set(&user, &balance);
            }
            pub fn set_global_balance(env: Env, balance: i128) {
                let user = Address::from_str("GABC");
                env.storage().instance().set(&user, &balance);
            }
        }
    "#;
    let rule = ShadowStorageRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("shadow_storage", violations_json(&violations));
    });
}

// ── reentrancy ────────────────────────────────────────────────────────────────

#[test]
fn sarif_reentrancy() {
    let source = r#"
        impl MyContract {
            pub fn withdraw(env: Env, amount: i128, recipient: Address) {
                env.storage().persistent().set(&symbol_short!("BAL"), &(amount - 10));
                env.invoke_contract::<()>(&recipient, &symbol_short!("recv"), vec![&env]);
            }
        }
    "#;
    let rule = ReentrancyRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "reentrancy must fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("reentrancy", violations_json(&violations));
    });
}

// ── unused_variable ───────────────────────────────────────────────────────────

#[test]
fn sarif_unused_variable() {
    let source = r#"
        impl MyContract {
            pub fn compute(env: Env, input: i128) -> i128 {
                let result = input * 2;
                let extra = 99i128;
                result
            }
        }
    "#;
    let rule = UnusedVariableRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "unused_variable must fire for 'extra'");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("unused_variable", violations_json(&violations));
    });
}

// ── truncation_bounds ─────────────────────────────────────────────────────────

#[test]
fn sarif_truncation_bounds() {
    let source = r#"
        impl MyContract {
            pub fn shrink(env: Env, big: i128) -> u32 {
                big as u32
            }
        }
    "#;
    let rule = TruncationBoundsRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "truncation_bounds must fire for i128 as u32");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("truncation_bounds", violations_json(&violations));
    });
}

// ── unchecked_external_call ───────────────────────────────────────────────────

#[test]
fn sarif_unchecked_external_call() {
    let source = r#"
        impl MyContract {
            pub fn call_other(env: Env, other: Address) {
                env.invoke_contract::<()>(&other, &symbol_short!("foo"), vec![&env]);
            }
        }
    "#;
    let rule = UncheckedExternalCallRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("unchecked_external_call", violations_json(&violations));
    });
}

// ── storage_update_state_check ────────────────────────────────────────────────

#[test]
fn sarif_storage_update_state_check() {
    let source = r#"
        impl MyContract {
            pub fn bump_counter(env: Env) {
                env.storage().instance().update(&symbol_short!("CTR"), |v: Option<u32>| -> Result<u32, ()> {
                    Ok(v.unwrap_or(0) + 1)
                });
            }
        }
    "#;
    let rule = StorageUpdateStateCheckRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("storage_update_state_check", violations_json(&violations));
    });
}

// ── variable_shadowing ────────────────────────────────────────────────────────

#[test]
fn sarif_variable_shadowing() {
    let source = r#"
        impl MyContract {
            pub fn compute(env: Env, x: i128) -> i128 {
                let x = x * 2;
                let x = x + 1;
                x
            }
        }
    "#;
    let rule = VariableShadowingRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("variable_shadowing", violations_json(&violations));
    });
}

// ── missing_state_event ───────────────────────────────────────────────────────

#[test]
fn sarif_missing_state_event() {
    let source = r#"
        impl MyContract {
            pub fn update_config(env: Env, new_fee: u32) {
                env.storage().instance().set(&symbol_short!("FEE"), &new_fee);
            }
        }
    "#;
    let rule = MissingStateEventRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("missing_state_event", violations_json(&violations));
    });
}

// ── instance_storage_misuse ───────────────────────────────────────────────────

#[test]
fn sarif_instance_storage_misuse() {
    let source = r#"
        impl MyContract {
            pub fn set_balance(env: Env, user: Address, amount: i128) {
                env.storage().instance().set(&user, &amount);
            }
        }
    "#;
    let rule = InstanceStorageMisuseRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("instance_storage_misuse", violations_json(&violations));
    });
}

// ── ledger_size (no-violation baseline) ──────────────────────────────────────

#[test]
fn sarif_ledger_size_clean() {
    let source = r#"
        #[contracttype]
        pub struct SmallEntry {
            pub value: u32,
        }

        impl MyContract {
            pub fn store(env: Env, v: u32) {
                env.storage().persistent().set(&symbol_short!("V"), &v);
            }
        }
    "#;
    let rule = LedgerSizeRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("ledger_size_clean", violations_json(&violations));
    });
}
