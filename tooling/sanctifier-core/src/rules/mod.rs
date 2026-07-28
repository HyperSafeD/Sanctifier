pub mod allowance_race;
pub mod arg_dos;
pub mod arithmetic_overflow;
pub mod auth_gap;
pub mod balance_equality;
pub mod division_by_zero;
pub mod edge_amount;
pub mod error_code_collision;
pub mod excessive_clone;
pub mod fee_rounding;
pub mod hardcoded_addr;
pub mod init_hardcoded_admin;
pub mod ledger_seconds;
pub mod ledger_size;
pub mod missing_ttl;
pub mod panic_detection;
pub mod reserve_withdrawal;
pub mod sanct_unwrap;
pub mod shift_overflow;
pub mod state_write_in_view;
pub mod tier_boundary_off_by_one;
pub mod unbounded_storage;
pub mod unhandled_result;
pub mod unsigned_underflow;
pub mod unused_variable;
pub mod view_panic;

use serde::Serialize;
use std::any::Any;

pub trait Rule: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn check(&self, source: &str) -> Vec<RuleViolation>;
    fn fix(&self, _source: &str) -> Vec<Patch> {
        vec![]
    }
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Patch {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub replacement: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleViolation {
    pub rule_name: String,
    pub severity: Severity,
    pub message: String,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub patches: Vec<Patch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl RuleViolation {
    pub fn new(rule_name: &str, severity: Severity, message: String, location: String) -> Self {
        Self {
            rule_name: rule_name.to_string(),
            severity,
            message,
            location,
            suggestion: None,
            patches: vec![],
        }
    }

    pub fn with_patches(mut self, patches: Vec<Patch>) -> Self {
        self.patches = patches;
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

pub struct RuleRegistry {
    pub(crate) rules: Vec<Box<dyn Rule>>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn register<R: Rule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    pub fn run_all(&self, source: &str) -> Vec<RuleViolation> {
        let mut violations: Vec<RuleViolation> = self
            .rules
            .iter()
            .flat_map(|rule| rule.check(source))
            .collect();

        // Macro-expansion-aware pass: analyse logic hidden behind simple local
        // `macro_rules!` wrappers so it isn't a false negative. The expansion is
        // additive — findings already visible in the original source are
        // de-duplicated by (rule, message), and code with no expandable macros
        // is left completely unchanged.
        if let Some(expanded) = crate::macro_expand::expand_local_macros(source) {
            let mut seen: std::collections::HashSet<(String, String)> = violations
                .iter()
                .map(|v| (v.rule_name.clone(), v.message.clone()))
                .collect();
            for rule in &self.rules {
                for v in rule.check(&expanded) {
                    if seen.insert((v.rule_name.clone(), v.message.clone())) {
                        violations.push(v);
                    }
                }
            }
        }

        violations
    }

    pub fn run_by_name(&self, source: &str, name: &str) -> Vec<RuleViolation> {
        self.rules
            .iter()
            .filter(|rule| rule.name() == name)
            .flat_map(|rule| rule.check(source))
            .collect()
    }

    pub fn available_rules(&self) -> Vec<&str> {
        self.rules.iter().map(|rule| rule.name()).collect()
    }

    pub fn with_default_rules() -> Self {
        let mut registry = Self::new();
        registry.register(auth_gap::AuthGapRule::new());
        registry.register(auth_gap::VisibilityLeakRule::new());
        registry.register(ledger_size::LedgerSizeRule::new());
        registry.register(panic_detection::PanicDetectionRule::new());
        registry.register(arithmetic_overflow::ArithmeticOverflowRule::new());
        registry.register(unhandled_result::UnhandledResultRule::new());
        registry.register(unused_variable::UnusedVariableRule::new());
        // New hygiene rules
        registry.register(hardcoded_addr::HardcodedAddrRule::new());
        registry.register(error_code_collision::ErrorCodeCollisionRule::new());
        registry.register(edge_amount::EdgeAmountRule::new());
        registry.register(balance_equality::BalanceEqualityRule::new());
        registry.register(fee_rounding::FeeRoundingRule::new());
        registry.register(excessive_clone::ExcessiveCloneRule::new());
        registry.register(missing_ttl::MissingTtlRule::new());
        registry.register(arg_dos::ArgDosRule::new());
        registry.register(sanct_unwrap::SanctUnwrapRule::new());
        registry.register(init_hardcoded_admin::InitHardcodedAdminRule::new());
        registry.register(shift_overflow::ShiftOverflowRule::new());
        registry.register(unbounded_storage::UnboundedStorageRule::new());
        registry.register(view_panic::ViewPanicRule::new());
        registry.register(allowance_race::AllowanceRaceRule::new());
        registry.register(state_write_in_view::StateWriteInViewRule::new());
        registry.register(division_by_zero::DivisionByZeroRule::new());
        registry.register(unsigned_underflow::UnsignedUnderflowRule::new());
        registry.register(ledger_seconds::LedgerSecondsRule::new());
        registry.register(tier_boundary_off_by_one::TierBoundaryOffByOneRule::new());
        registry.register(reserve_withdrawal::ReserveWithdrawalRule::new());
        registry
    }
}
