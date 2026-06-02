//! Pluggable rule system for static analysis.
//!
//! Implement the [`Rule`] trait to create a custom check, then register it
//! with [`RuleRegistry::register`].

/// Unchecked arithmetic detection.
pub mod arithmetic_overflow;
/// Missing authorization checks.
pub mod auth_gap;
/// Instance storage misuse — per-user data stored in Instance instead of Persistent.
pub mod instance_storage_misuse;
/// Ledger entry size analysis.
pub mod ledger_size;
/// Missing state event emission.
pub mod missing_state_event;
/// Panic / unwrap detection.
pub mod panic_detection;
/// Reentrancy vulnerability detection and auto-fix.
pub mod reentrancy;
/// Shadow storage pattern detection.
pub mod shadow_storage;
/// Detect usage of env.storage().instance().update() without state check.
pub mod storage_update_state_check;
/// Integer truncation and unchecked bounds detection.
pub mod truncation_bounds;
/// Unchecked external call detection.
pub mod unchecked_external_call;
/// Unhandled `Result` values.
pub mod unhandled_result;
/// Unsafe PRNG usage in state-critical code.
pub mod unsafe_prng;
/// Unused local variables.
pub mod unused_variable;
/// Direct xdr::ScVal raw construction detection.
pub mod xdr_raw_construction;

/// Variable shadowing in nested scopes.
pub mod variable_shadowing;
/// Raw `invoke_contract` call without `try_invoke_contract` error handling.
pub mod raw_invoke_contract;
/// `#[test]` functions that never reference a `ContractClient`.
pub mod shallow_test;
/// transfer_from-style flows that consume 'from' balance without allowance checks.
pub mod transfer_from_no_allowance;
/// Persistent/Temporary storage writes without a TTL bump (extend_ttl).
pub mod missing_ttl_bump;
/// Taint propagation through tuple and struct destructures.
pub mod taint_propagation;
/// Static reentrancy — external call before state write (complement to runtime guard).
pub mod static_reentrancy;
/// Soroban SDK v22 deprecated storage/deployment API patterns.
pub mod deprecated_sdk_usage;
/// Detect env.ledger().timestamp() used as entropy for randomness.
pub mod timestamp_randomness;
/// require_auth used instead of require_auth_for_args in multi-arg admin operations.
pub mod require_auth_for_args;
use serde::Serialize;
use std::any::Any;

/// A single analysis rule.
///
/// Every rule must be `Send + Sync` so that it can be shared across rayon
/// threads.
pub trait Rule: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe {
    /// Unique machine-readable name (e.g. `"auth_gap"`).
    fn name(&self) -> &str;
    /// Human-readable description.
    fn description(&self) -> &str;
    /// Run the check and return all violations.
    fn check(&self, source: &str) -> Vec<RuleViolation>;
    /// Optionally produce auto-fix patches.
    fn fix(&self, _source: &str) -> Vec<Patch> {
        vec![]
    }
    /// Down-cast helper.
    fn as_any(&self) -> &dyn Any;
}

/// A source-level text replacement.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct Patch {
    /// Start line (1-based).
    pub start_line: usize,
    /// Start column (0-based).
    pub start_column: usize,
    /// End line (1-based).
    pub end_line: usize,
    /// End column (0-based).
    pub end_column: usize,
    /// Replacement text.
    pub replacement: String,
    /// Human-readable description.
    pub description: String,
}

/// A single violation emitted by a [`Rule`].
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RuleViolation {
    /// Name of the rule that fired.
    pub rule_name: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Source location.
    pub location: String,
    /// Optional suggestion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Optional auto-fix patches.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub patches: Vec<Patch>,
}

/// Severity level of a rule violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Severity {
    /// Hard error — blocks CI.
    Error,
    /// Warning — should be addressed.
    Warning,
    /// Informational.
    Info,
}

impl RuleViolation {
    /// Create a new violation.
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

    /// Attach auto-fix patches.
    pub fn with_patches(mut self, patches: Vec<Patch>) -> Self {
        self.patches = patches;
        self
    }

    /// Attach a human-readable suggestion.
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// A registry of [`Rule`] implementations.
///
/// Use [`RuleRegistry::with_default_rules`] to get the built-in set.
pub struct RuleRegistry {
    pub(crate) rules: Vec<Box<dyn Rule>>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

impl RuleRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a rule.
    pub fn register<R: Rule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    /// Run every registered rule against `source`.
    pub fn run_all(&self, source: &str) -> Vec<RuleViolation> {
        self.rules
            .iter()
            .flat_map(|rule| rule.check(source))
            .collect()
    }

    /// Run a single rule by name.
    pub fn run_by_name(&self, source: &str, name: &str) -> Vec<RuleViolation> {
        self.rules
            .iter()
            .filter(|rule| rule.name() == name)
            .flat_map(|rule| rule.check(source))
            .collect()
    }

    /// List the names of all registered rules.
    pub fn available_rules(&self) -> Vec<&str> {
        self.rules.iter().map(|rule| rule.name()).collect()
    }

    /// Create a registry pre-loaded with all built-in rules.
    pub fn with_default_rules() -> Self {
        let mut registry = Self::new();
        registry.register(auth_gap::AuthGapRule::new());
        registry.register(ledger_size::LedgerSizeRule::new());
        registry.register(panic_detection::PanicDetectionRule::new());
        registry.register(arithmetic_overflow::ArithmeticOverflowRule::new());
        registry.register(unhandled_result::UnhandledResultRule::new());
        registry.register(unused_variable::UnusedVariableRule::new());
        registry.register(shadow_storage::ShadowStorageRule::new());
        registry.register(xdr_raw_construction::XdrRawConstructionRule::new());
        registry.register(storage_update_state_check::StorageUpdateStateCheckRule::new());
        registry.register(reentrancy::ReentrancyRule::new());
        registry.register(truncation_bounds::TruncationBoundsRule::new());
        registry.register(unsafe_prng::UnsafePrngRule::new());
        registry.register(variable_shadowing::VariableShadowingRule::new());
        registry.register(unchecked_external_call::UncheckedExternalCallRule::new());
        registry.register(missing_state_event::MissingStateEventRule::new());
        registry.register(instance_storage_misuse::InstanceStorageMisuseRule::new());
        registry.register(raw_invoke_contract::RawInvokeContractRule::new());
        registry.register(shallow_test::ShallowTestRule::new());
        registry.register(transfer_from_no_allowance::TransferFromNoAllowanceRule::new());
        registry.register(missing_ttl_bump::MissingTtlBumpRule::new());
        registry.register(taint_propagation::TaintPropagationRule::new());
        registry.register(static_reentrancy::StaticReentrancyRule::new());
        registry.register(deprecated_sdk_usage::DeprecatedSdkUsageRule::new());
        registry.register(timestamp_randomness::TimestampRandomnessRule::new());
        registry.register(require_auth_for_args::RequireAuthForArgsRule::new());
        registry
    }
}

#[cfg(test)]
mod crlf_tests;
