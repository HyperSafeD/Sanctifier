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
    arithmetic_overflow::ArithmeticOverflowRule, auth_gap::AuthGapRule,
    instance_storage_misuse::InstanceStorageMisuseRule, ledger_size::LedgerSizeRule,
    missing_state_event::MissingStateEventRule, panic_detection::PanicDetectionRule,
    reentrancy::ReentrancyRule, shadow_storage::ShadowStorageRule,
    storage_update_state_check::StorageUpdateStateCheckRule,
    truncation_bounds::TruncationBoundsRule, unchecked_external_call::UncheckedExternalCallRule,
    unhandled_result::UnhandledResultRule, unsafe_prng::UnsafePrngRule,
    unused_variable::UnusedVariableRule, variable_shadowing::VariableShadowingRule,
    zk_double_spend_risk::ZkDoubleSpendRiskRule,
    zk_hardcoded_trusted_setup::ZkHardcodedTrustedSetupRule,
    zk_missing_constraint::ZkMissingConstraintRule,
    zk_missing_public_input_binding::ZkMissingPublicInputBindingRule,
    zk_missing_vk_integrity_check::ZkMissingVkIntegrityCheckRule,
    zk_verification_result_ignored::ZkVerificationResultIgnoredRule,
    zk_verifier_skippable::ZkVerifierSkippableRule, Rule, RuleViolation,
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
    assert!(
        !violations.is_empty(),
        "panic_detection must fire for unwrap"
    );
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
    assert!(
        !violations.is_empty(),
        "arithmetic_overflow must fire for bare +"
    );
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
    assert!(
        !violations.is_empty(),
        "unhandled_result must fire for ignored try_*"
    );
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
    assert!(
        !violations.is_empty(),
        "unused_variable must fire for 'extra'"
    );
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
    assert!(
        !violations.is_empty(),
        "truncation_bounds must fire for i128 as u32"
    );
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

// ── Z-series snapshot tests (#1224) ──────────────────────────────────────────

// ── Z001: zk_missing_constraint (triggering fixture) ─────────────────────────

#[test]
fn sarif_zk_missing_constraint_trigger() {
    let source = r#"
        impl ShieldedContract {
            pub fn withdraw(env: Env, proof: Vec<u8>, amount: i128) {
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkMissingConstraintRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "zk_missing_constraint must fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_missing_constraint_trigger", violations_json(&violations));
    });
}

#[test]
fn sarif_zk_missing_constraint_clean() {
    let source = r#"
        impl ShieldedContract {
            pub fn withdraw(env: Env, proof: Vec<u8>, amount: i128) {
                verify_proof(&env, &proof).expect("invalid proof");
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkMissingConstraintRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_missing_constraint_clean", violations_json(&violations));
    });
}

// ── Z004: zk_verifier_skippable (triggering fixture) ─────────────────────────

#[test]
fn sarif_zk_verifier_skippable_trigger() {
    let source = r#"
        impl PrivateTransfer {
            pub fn transfer(env: Env, proof: Vec<u8>, use_zk: bool, amount: i128) {
                if use_zk {
                    verify_proof(&env, &proof);
                } else {
                    // verification bypassed
                }
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkVerifierSkippableRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "zk_verifier_skippable must fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_verifier_skippable_trigger", violations_json(&violations));
    });
}

#[test]
fn sarif_zk_verifier_skippable_clean() {
    let source = r#"
        impl PrivateTransfer {
            pub fn transfer(env: Env, proof: Vec<u8>, amount: i128) {
                verify_proof(&env, &proof).expect("invalid proof");
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkVerifierSkippableRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_verifier_skippable_clean", violations_json(&violations));
    });
}

// ── Z005: zk_double_spend_risk (triggering fixture) ──────────────────────────

#[test]
fn sarif_zk_double_spend_risk_trigger() {
    let source = r#"
        impl PrivatePool {
            pub fn withdraw(env: Env, proof: Vec<u8>, amount: i128) {
                verify_proof(&env, &proof).expect("bad proof");
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkDoubleSpendRiskRule::new();
    let violations = rule.check(source);
    assert!(!violations.is_empty(), "zk_double_spend_risk must fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_double_spend_risk_trigger", violations_json(&violations));
    });
}

#[test]
fn sarif_zk_double_spend_risk_clean() {
    let source = r#"
        impl PrivatePool {
            pub fn withdraw(env: Env, proof: Vec<u8>, nullifier: BytesN<32>, amount: i128) {
                let is_spent: bool = env.storage().persistent()
                    .get(&nullifier).unwrap_or(false);
                assert!(!is_spent, "nullifier already spent");
                verify_proof(&env, &proof).expect("bad proof");
                env.storage().persistent().set(&nullifier, &true);
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkDoubleSpendRiskRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_double_spend_risk_clean", violations_json(&violations));
    });
}

// ── Z013: zk_verification_result_ignored (triggering fixture) ────────────────

#[test]
fn sarif_zk_verification_result_ignored_trigger() {
    let source = r#"
        impl ShieldedTransfer {
            pub fn execute(env: Env, proof: Vec<u8>, amount: i128) {
                verify_proof(&env, &proof);
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkVerificationResultIgnoredRule::new();
    let violations = rule.check(source);
    assert!(
        !violations.is_empty(),
        "zk_verification_result_ignored must fire"
    );
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_verification_result_ignored_trigger", violations_json(&violations));
    });
}

#[test]
fn sarif_zk_verification_result_ignored_clean() {
    let source = r#"
        impl ShieldedTransfer {
            pub fn execute(env: Env, proof: Vec<u8>, amount: i128) {
                verify_proof(&env, &proof).expect("invalid proof");
                env.storage().persistent().set(&symbol_short!("BAL"), &amount);
            }
        }
    "#;
    let rule = ZkVerificationResultIgnoredRule::new();
    let violations = rule.check(source);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("zk_verification_result_ignored_clean", violations_json(&violations));
    });
}

// ── Z003 / Z004 / Z005 (#1199, #1200, #1201) ─────────────────────────────────
//
// These run against the checked-in fixtures rather than inline snippets, so a
// fixture and its rule cannot drift apart silently.

/// Load a fixture from `contracts/fixtures/finding-codes/`.
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/fixtures/finding-codes")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()))
}

/// The function names a rule fired on, sorted for stable comparison.
fn flagged_fns(violations: &[RuleViolation]) -> Vec<String> {
    let mut names: Vec<String> = violations
        .iter()
        .map(|v| v.location.split(':').next().unwrap_or("").to_string())
        .collect();
    names.sort();
    names
}

// ── Z003: missing_public_input_binding ───────────────────────────────────────

#[test]
fn sarif_z003_missing_public_input_binding_fixture() {
    let source = fixture("z003_missing_public_input_binding.rs");
    let violations = ZkMissingPublicInputBindingRule::new().check(&source);

    // Only the unbound-recipient function may fire; the three safe variants
    // (hash-bound, directly bound, and value-unused-after-verify) must not.
    assert_eq!(
        flagged_fns(&violations),
        vec!["withdraw_vulnerable"],
        "Z003 flagged the wrong set of functions: {violations:?}"
    );

    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "z003_missing_public_input_binding_trigger",
            violations_json(&violations)
        );
    });
}

#[test]
fn sarif_z003_clean() {
    let source = r#"
        impl Shielded {
            pub fn withdraw(env: Env, proof: Vec<u64>, recipient: Address, amount: i128) {
                let recipient_hash = hash_address(&env, &recipient);
                let public_inputs = vec![&env, recipient_hash, amount as u64];
                verify_proof(&env, &proof, &public_inputs);
                token_client.transfer(&env.current_contract_address(), &recipient, &amount);
            }
        }
    "#;
    let violations = ZkMissingPublicInputBindingRule::new().check(source);
    assert!(violations.is_empty(), "bound inputs must not fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "z003_missing_public_input_binding_clean",
            violations_json(&violations)
        );
    });
}

// ── Z004: hardcoded_trusted_setup ────────────────────────────────────────────

#[test]
fn sarif_z004_unverified_trusted_setup_fixture() {
    let source = fixture("z004_unverified_trusted_setup.rs");
    let violations = ZkHardcodedTrustedSetupRule::new().check(&source);

    // The two undocumented constants fire; the ceremony-documented keys, the
    // storage-key string, and the unrelated limit must not.
    assert_eq!(
        flagged_fns(&violations),
        vec!["TRUSTED_SETUP_PARAMS", "VK_ALPHA_G1_UNDOCUMENTED"],
        "Z004 flagged the wrong set of constants: {violations:?}"
    );

    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "z004_unverified_trusted_setup_trigger",
            violations_json(&violations)
        );
    });
}

#[test]
fn sarif_z004_clean() {
    let source = "\
// ceremony: perpetual powers-of-tau phase2, contribution #47
// transcript sha256: 3f7a1c04e5b28d9f6a1103bb77c4e2d5081aa93cf4be6710d2ac5f38e91b7c19
const VERIFYING_KEY: [u8; 4] = [0x01, 0x02, 0x03, 0x04];
";
    let violations = ZkHardcodedTrustedSetupRule::new().check(source);
    assert!(violations.is_empty(), "documented ceremony must not fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "z004_unverified_trusted_setup_clean",
            violations_json(&violations)
        );
    });
}

// ── Z005: missing_vk_integrity_check ─────────────────────────────────────────

#[test]
fn sarif_z005_missing_vk_integrity_check_fixture() {
    let source = fixture("z005_missing_vk_integrity_check.rs");
    let violations = ZkMissingVkIntegrityCheckRule::new().check(&source);

    // Both unchecked storage-loaded paths fire; the hash-asserted path, the
    // immutable constant key, and the plain getter must not.
    assert_eq!(
        flagged_fns(&violations),
        vec!["verify_inline_vulnerable", "verify_vulnerable"],
        "Z005 flagged the wrong set of functions: {violations:?}"
    );

    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "z005_missing_vk_integrity_check_trigger",
            violations_json(&violations)
        );
    });
}

#[test]
fn sarif_z005_clean() {
    let source = r#"
        impl Verifier {
            pub fn verify(env: Env, proof: Vec<u8>, inputs: Vec<u64>) -> bool {
                let vk: BytesN<64> = env.storage().persistent().get(&DataKey::VerifyingKey).unwrap();
                let expected: BytesN<32> = env.storage().persistent().get(&DataKey::VkHash).unwrap();
                assert_eq!(env.crypto().sha256(&Bytes::from_slice(&env, vk.as_ref())), expected, "vk integrity");
                groth16_verify(vk.as_ref(), &proof, &inputs)
            }
        }
    "#;
    let violations = ZkMissingVkIntegrityCheckRule::new().check(source);
    assert!(violations.is_empty(), "checked VK must not fire");
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "z005_missing_vk_integrity_check_clean",
            violations_json(&violations)
        );
    });
}
