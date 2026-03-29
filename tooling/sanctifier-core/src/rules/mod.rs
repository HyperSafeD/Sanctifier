//! Pluggable rule system for static analysis.
//!
//! Implement the [`Rule`] trait to create a custom check, then register it
//! with [`RuleRegistry::register`].

/// Unchecked arithmetic detection.
pub mod arithmetic_overflow;
/// Missing authorization checks.
pub mod auth_gap;
/// Ledger entry size analysis.
pub mod ledger_size;
/// Panic / unwrap detection.
pub mod panic_detection;
/// Reentrancy vulnerability detection and auto-fix.
pub mod reentrancy;
/// Shadow storage pattern detection.
pub mod shadow_storage;
/// Integer truncation and unchecked bounds detection.
pub mod truncation_bounds;
/// Unhandled `Result` values.
pub mod unhandled_result;
/// Unused local variables.
pub mod unused_variable;
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
#[derive(Debug, Clone, Serialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
        registry.register(reentrancy::ReentrancyRule::new());
        registry.register(truncation_bounds::TruncationBoundsRule::new());
        registry
    }
}
