pub mod arithmetic_overflow;
pub mod auth_gap;
pub mod ledger_size;
pub mod panic_detection;
pub mod unhandled_result;

use serde::Serialize;
use std::any::Any;

pub trait Rule: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn check(&self, source: &str) -> Vec<RuleViolation>;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleViolation {
    pub rule_name: String,
    pub severity: Severity,
    pub message: String,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
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
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
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
        self.rules
            .iter()
            .flat_map(|rule| rule.check(source))
            .collect()
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
        registry.register(ledger_size::LedgerSizeRule::new());
        registry.register(panic_detection::PanicDetectionRule::new());
        registry.register(arithmetic_overflow::ArithmeticOverflowRule::new());
        registry.register(unhandled_result::UnhandledResultRule::new());
        registry
    }
}
