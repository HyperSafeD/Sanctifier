//! Golden snapshot tests for every detector.
//!
//! Each detector gets a dedicated fixture under `tests/fixtures/detectors/` and
//! a reviewed `insta` snapshot of the `RuleViolation`s it produces. CI runs these
//! as part of the normal test suite, so any unintended change to a detector's
//! output fails the build until the snapshot is re-reviewed.
//!
//! Workflow:
//!   * `cargo insta test`   — run the snapshot tests.
//!   * `cargo insta review` — interactively accept/reject pending changes.
//!   * `cargo insta accept` — accept all pending changes (use with care).
//!
//! See `tooling/sanctifier-core/tests/README.md` for the full guide.

use sanctifier_core::rules::auth_gap::VisibilityLeakRule;
use sanctifier_core::rules::{
    allowance_race::AllowanceRaceRule, arg_dos::ArgDosRule,
    arithmetic_overflow::ArithmeticOverflowRule, auth_gap::AuthGapRule,
    balance_equality::BalanceEqualityRule, division_by_zero::DivisionByZeroRule,
    edge_amount::EdgeAmountRule, error_code_collision::ErrorCodeCollisionRule,
    excessive_clone::ExcessiveCloneRule, fee_rounding::FeeRoundingRule,
    hardcoded_addr::HardcodedAddrRule, init_hardcoded_admin::InitHardcodedAdminRule,
    ledger_seconds::LedgerSecondsRule, ledger_size::LedgerSizeRule, missing_ttl::MissingTtlRule,
    panic_detection::PanicDetectionRule, sanct_unwrap::SanctUnwrapRule,
    shift_overflow::ShiftOverflowRule, state_write_in_view::StateWriteInViewRule,
    tier_boundary_off_by_one::TierBoundaryOffByOneRule, unbounded_storage::UnboundedStorageRule,
    unhandled_result::UnhandledResultRule, unsigned_underflow::UnsignedUnderflowRule,
    unused_variable::UnusedVariableRule, view_panic::ViewPanicRule, Rule, RuleRegistry,
};

/// Run a detector against its fixture and snapshot the resulting findings.
///
/// The snapshot name is set explicitly so each detector maps to a stable,
/// human-readable snapshot file (`snapshots/detector_snapshots__<name>.snap`).
fn assert_detector_snapshot(name: &str, rule: &dyn Rule, fixture: &str) {
    let findings = rule.check(fixture);
    insta::assert_yaml_snapshot!(name, findings);
}

#[test]
fn snapshot_auth_gap() {
    assert_detector_snapshot(
        "auth_gap",
        &AuthGapRule::new(),
        include_str!("fixtures/detectors/auth_gap.rs"),
    );
}

#[test]
fn snapshot_sanct_visibility() {
    assert_detector_snapshot(
        "sanct_visibility",
        &VisibilityLeakRule::new(),
        include_str!("fixtures/detectors/sanct_visibility.rs"),
    );
}

#[test]
fn snapshot_arithmetic_overflow() {
    assert_detector_snapshot(
        "arithmetic_overflow",
        &ArithmeticOverflowRule::new(),
        include_str!("fixtures/detectors/arithmetic_overflow.rs"),
    );
}

#[test]
fn snapshot_unhandled_result() {
    assert_detector_snapshot(
        "unhandled_result",
        &UnhandledResultRule::new(),
        include_str!("fixtures/detectors/unhandled_result.rs"),
    );
}

#[test]
fn snapshot_unused_variable() {
    assert_detector_snapshot(
        "unused_variable",
        &UnusedVariableRule::new(),
        include_str!("fixtures/detectors/unused_variable.rs"),
    );
}

#[test]
fn snapshot_panic_detection() {
    assert_detector_snapshot(
        "panic_detection",
        &PanicDetectionRule::new(),
        include_str!("fixtures/detectors/panic_detection.rs"),
    );
}

#[test]
fn snapshot_ledger_size() {
    assert_detector_snapshot(
        "ledger_size",
        &LedgerSizeRule::new(),
        include_str!("fixtures/detectors/ledger_size.rs"),
    );
}

#[test]
fn snapshot_hardcoded_addr() {
    assert_detector_snapshot(
        "hardcoded_addr",
        &HardcodedAddrRule::new(),
        include_str!("fixtures/detectors/hardcoded_addr.rs"),
    );
}

#[test]
fn snapshot_error_code_collision() {
    assert_detector_snapshot(
        "error_code_collision",
        &ErrorCodeCollisionRule::new(),
        include_str!("fixtures/detectors/error_code_collision.rs"),
    );
}

#[test]
fn snapshot_edge_amount() {
    assert_detector_snapshot(
        "edge_amount",
        &EdgeAmountRule::new(),
        include_str!("fixtures/detectors/edge_amount.rs"),
    );
}

#[test]
fn snapshot_balance_equality() {
    assert_detector_snapshot(
        "balance_equality",
        &BalanceEqualityRule::new(),
        include_str!("fixtures/detectors/balance_equality.rs"),
    );
}

#[test]
fn snapshot_excessive_clone() {
    assert_detector_snapshot(
        "excessive_clone",
        &ExcessiveCloneRule::new(),
        include_str!("fixtures/detectors/excessive_clone.rs"),
    );
}

#[test]
fn snapshot_fee_rounding() {
    assert_detector_snapshot(
        "fee_rounding",
        &FeeRoundingRule::new(),
        include_str!("fixtures/detectors/fee_rounding.rs"),
    );
}

#[test]
fn snapshot_ledger_seconds() {
    assert_detector_snapshot(
        "ledger_seconds",
        &LedgerSecondsRule::new(),
        include_str!("fixtures/detectors/ledger_seconds.rs"),
    );
}

#[test]
fn snapshot_missing_ttl() {
    assert_detector_snapshot(
        "missing_ttl",
        &MissingTtlRule::new(),
        include_str!("fixtures/detectors/missing_ttl.rs"),
    );
}

#[test]
fn snapshot_arg_dos() {
    assert_detector_snapshot(
        "arg_dos",
        &ArgDosRule::new(),
        include_str!("fixtures/detectors/arg_dos.rs"),
    );
}

#[test]
fn snapshot_sanct_unwrap() {
    assert_detector_snapshot(
        "sanct_unwrap",
        &SanctUnwrapRule::new(),
        include_str!("fixtures/detectors/sanct_unwrap.rs"),
    );
}

#[test]
fn snapshot_init_hardcoded_admin() {
    assert_detector_snapshot(
        "init_hardcoded_admin",
        &InitHardcodedAdminRule::new(),
        include_str!("fixtures/detectors/init_hardcoded_admin.rs"),
    );
}

#[test]
fn snapshot_unbounded_storage() {
    assert_detector_snapshot(
        "unbounded_storage",
        &UnboundedStorageRule::new(),
        include_str!("fixtures/detectors/unbounded_storage.rs"),
    );
}

#[test]
fn snapshot_state_write_in_view() {
    assert_detector_snapshot(
        "state_write_in_view",
        &StateWriteInViewRule::new(),
        include_str!("fixtures/detectors/state_write_in_view.rs"),
    );
}

#[test]
fn snapshot_view_panic() {
    assert_detector_snapshot(
        "view_panic",
        &ViewPanicRule::new(),
        include_str!("fixtures/detectors/view_panic.rs"),
    );
}

#[test]
fn snapshot_allowance_race() {
    assert_detector_snapshot(
        "allowance_race",
        &AllowanceRaceRule::new(),
        include_str!("fixtures/detectors/allowance_race.rs"),
    );
}

#[test]
fn snapshot_division_by_zero() {
    assert_detector_snapshot(
        "division_by_zero",
        &DivisionByZeroRule::new(),
        include_str!("fixtures/detectors/division_by_zero.rs"),
    );
}

#[test]
fn snapshot_tier_boundary_off_by_one() {
    assert_detector_snapshot(
        "tier_boundary_off_by_one",
        &TierBoundaryOffByOneRule::new(),
        include_str!("fixtures/detectors/tier_boundary_off_by_one.rs"),
    );
}

#[test]
fn snapshot_shift_overflow() {
    assert_detector_snapshot(
        "shift_overflow",
        &ShiftOverflowRule::new(),
        include_str!("fixtures/detectors/shift_overflow.rs"),
    );
}

#[test]
fn snapshot_unsigned_underflow() {
    assert_detector_snapshot(
        "unsigned_underflow",
        &UnsignedUnderflowRule::new(),
        include_str!("fixtures/detectors/unsigned_underflow.rs"),
    );
}

#[test]
fn unbounded_storage_detector_flags_only_uncapped_persistent_growth() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/unbounded_storage.rs"),
        "unbounded_storage",
    );

    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert!(findings
        .iter()
        .all(|finding| finding.rule_name == "SANCT_UNBOUNDED_STORAGE"));
    assert!(findings.iter().any(
        |finding| finding.location.contains("register") && finding.message.contains("members")
    ));
    assert!(findings
        .iter()
        .any(|finding| finding.location.contains("record_score")
            && finding.message.contains("scores")));
}

#[test]
fn view_panic_detector_flags_only_view_entrypoints() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/view_panic.rs"),
        "view_panic",
    );

    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert!(findings
        .iter()
        .all(|finding| finding.rule_name == "SANCT_VIEW_PANIC"));
    assert!(findings.iter().any(|f| f.location.contains("get_price")));
    assert!(findings.iter().any(|f| f.location.contains("get_holder")));
}

#[test]
fn allowance_race_detector_is_registered_in_default_rules() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/allowance_race.rs"),
        "allowance_race",
    );

    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert_eq!(findings[0].rule_name, "SANCT_ALLOWANCE_RACE");
    assert!(findings[0].location.contains("approve"));
}

#[test]
fn state_write_in_view_detector_is_registered_in_default_rules() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/state_write_in_view.rs"),
        "state_write_in_view",
    );

    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert!(findings
        .iter()
        .all(|finding| finding.rule_name == "SANCT_STATE_WRITE_IN_VIEW"));
}

#[test]
fn shift_overflow_detector_is_registered_in_default_rules() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/shift_overflow.rs"),
        "shift_overflow",
    );

    assert_eq!(findings.len(), 3, "{findings:#?}");
    assert!(findings
        .iter()
        .all(|finding| finding.rule_name == "SANCT_SHIFT_OVERFLOW"));
}

#[test]
fn arg_dos_detector_flags_only_uncapped_argument_iteration() {
    let findings = RuleRegistry::with_default_rules()
        .run_by_name(include_str!("fixtures/detectors/arg_dos.rs"), "arg_dos");

    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert_eq!(findings[0].rule_name, "SANCT_ARG_DOS");
    assert!(findings[0].message.contains("recipients"));
    assert!(findings[0].location.contains("uncapped_vec_airdrop"));
}

#[test]
fn sanct_unwrap_detector_is_registered_in_default_rules() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/sanct_unwrap.rs"),
        "sanct_unwrap",
    );

    assert_eq!(findings.len(), 3, "{findings:#?}");
    assert!(findings
        .iter()
        .all(|finding| finding.rule_name == "SANCT_UNWRAP"));
}

#[test]
fn sanct_visibility_flags_only_the_exposed_unauthenticated_helper() {
    let findings = RuleRegistry::with_default_rules().run_by_name(
        include_str!("fixtures/detectors/sanct_visibility.rs"),
        "sanct_visibility",
    );

    assert_eq!(findings.len(), 8, "{findings:#?}");
    assert!(findings
        .iter()
        .all(|finding| finding.rule_name == "SANCT_VISIBILITY"));

    let messages = findings
        .iter()
        .map(|finding| finding.message.as_str())
        .collect::<Vec<_>>();
    assert!(messages
        .iter()
        .any(|message| message.contains("_set_balance")));
    assert!(messages
        .iter()
        .any(|message| message.contains("_set_balance_conditionally")));
    assert!(messages
        .iter()
        .any(|message| message.contains("helper_increment_balance")));
    assert!(messages
        .iter()
        .any(|message| message.contains("_set_balance_after_validation")));
    assert!(messages
        .iter()
        .any(|message| message.contains("helper_set_via_storage_alias")));
    assert!(messages
        .iter()
        .any(|message| message.contains("helper_set_via_external_storage")));
    assert!(messages
        .iter()
        .any(|message| message.contains("helper_set_after_nested_loop")));
    assert!(messages
        .iter()
        .any(|message| message.contains("internal_set_flag")));
}

#[test]
fn snapshot_reserve_withdrawal() {
    assert_detector_snapshot(
        "reserve_withdrawal",
        &sanctifier_core::rules::reserve_withdrawal::ReserveWithdrawalRule::new(),
        include_str!("fixtures/detectors/reserve_withdrawal.rs"),
    );
}
