//! **sanctifier-core** — static-analysis engine for Stellar Soroban smart
//! contracts.
//!
//! This crate provides the [`Analyzer`] entry-point together with a
//! [`RuleRegistry`] of pluggable rules.  Every finding is tagged with a
//! canonical code from the [`finding_codes`] module (`S000` – `S012`).
//!
//! # JSON output schema
//!
//! When the CLI is invoked with `--format json` the output conforms to the
//! JSON Schema (draft-07) published at
//! `schemas/analysis-output.json` in the repository root.  The schema is
//! versioned via a `schema_version` field in every report and validated in CI.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use sanctifier_core::{Analyzer, SanctifyConfig};
//!
//! let analyzer = Analyzer::new(SanctifyConfig::default());
//! let warnings = analyzer.analyze_ledger_size(source_code);
//! ```

#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::panic::catch_unwind;

/// Contract complexity metrics and reports.
pub mod complexity;
/// Canonical finding codes (`S000` – `S012`) emitted by every analysis pass.
pub mod finding_codes;
/// Gas / instruction-cost estimation heuristics.
pub mod gas_estimator;
/// (Reserved) Gas report rendering.
pub(crate) mod gas_report;
/// Automatic patch application.
pub mod patcher;
/// Pluggable rule system ([`Rule`] trait, [`RuleRegistry`], built-in rules).
pub mod rules;
/// SEP-41 token-interface verification.
pub mod sep41;
/// Z3 SMT solver integration for formal verification.
/// Only available when the `smt` feature is enabled (default).
#[cfg(feature = "smt")]
pub mod smt;
/// Stub SMT types used when the `smt` feature is disabled.
#[cfg(not(feature = "smt"))]
pub mod smt {
    use serde::Serialize;

    /// Placeholder invariant issue type for builds without SMT support.
    #[derive(Debug, Serialize, Clone, Default)]
    pub struct SmtInvariantIssue {
        /// Function under verification.
        pub function_name: String,
        /// Human-readable description of the violation.
        pub description: String,
        /// Source location.
        pub location: String,
    }
}
/// Storage-key collision detection (internal).
mod storage_collision;
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{parse_str, Fields, File, Item, Meta, Type};

pub use complexity::{analyze_complexity, analyze_complexity_from_source, render_text_report};
pub use rules::{Rule, RuleRegistry, RuleViolation, Severity};
pub use sep41::{Sep41Issue, Sep41IssueKind, Sep41VerificationReport};
pub use smt::SmtInvariantIssue;

// Redundant imports removed
use crate::rules::arithmetic_overflow::ArithVisitor;

const DEFAULT_STRICT_THRESHOLD: f64 = 0.9;
fn with_panic_guard<F, R>(f: F) -> R
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
    R: Default,
{
    catch_unwind(f).unwrap_or_default()
}

// ── Existing types ────────────────────────────────────────────────────────────

/// Severity of a ledger size warning.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[non_exhaustive]
pub enum SizeWarningLevel {
    /// Size exceeds the ledger entry limit (e.g. 64KB).
    ExceedsLimit,
    /// Size is approaching the limit (configurable threshold, e.g. 80%).
    ApproachingLimit,
}

/// A warning about a `#[contracttype]` whose estimated serialised size is
/// close to or exceeds the ledger entry limit.
#[derive(Debug, Serialize, Clone)]
pub struct SizeWarning {
    /// Name of the struct or enum that was analysed.
    pub struct_name: String,
    /// Estimated size in bytes.
    pub estimated_size: usize,
    /// The configured ledger-entry limit in bytes.
    pub limit: usize,
    /// Whether the size exceeds or merely approaches the limit.
    pub level: SizeWarningLevel,
}

/// A `panic!`, `.unwrap()`, or `.expect()` call found inside a contract
/// function.
#[derive(Debug, Serialize, Clone)]
pub struct PanicIssue {
    /// The contract function containing the panic-path.
    pub function_name: String,
    /// One of `"panic!"`, `"unwrap"`, or `"expect"`.
    pub issue_type: String,
    /// Human-readable location string.
    pub location: String,
}

// ── UnsafePattern types (visitor-based panic/unwrap scanning) ─────────────────

/// The kind of unsafe pattern detected by [`Analyzer::analyze_unsafe_patterns`].
#[derive(Debug, Serialize, Clone)]
#[non_exhaustive]
pub enum PatternType {
    /// A `panic!()` macro invocation.
    Panic,
    /// A `.unwrap()` call.
    Unwrap,
    /// A `.expect("…")` call.
    Expect,
}

/// An unsafe pattern (`panic!`, `.unwrap()`, `.expect()`) together with its
/// source location.
#[derive(Debug, Serialize, Clone)]
pub struct UnsafePattern {
    /// The kind of pattern.
    pub pattern_type: PatternType,
    /// 1-based line number.
    pub line: usize,
    /// Source snippet around the pattern.
    pub snippet: String,
}

// ── Upgrade analysis types ────────────────────────────────────────────────────

/// A single finding related to contract upgrade / admin mechanisms.
#[derive(Debug, Serialize, Clone)]
pub struct UpgradeFinding {
    /// Broad category of the finding.
    pub category: UpgradeCategory,
    /// Name of the related function, if any.
    pub function_name: Option<String>,
    /// Human-readable location.
    pub location: String,
    /// Description of the finding.
    pub message: String,
    /// Actionable suggestion.
    pub suggestion: String,
}

/// Category of an upgrade-safety finding.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UpgradeCategory {
    /// Admin-only access control.
    AdminControl,
    /// Timelock mechanism.
    Timelock,
    /// Initialisation pattern.
    InitPattern,
    /// Storage layout concern.
    StorageLayout,
    /// Governance / upgrade mechanism.
    Governance,
}

/// Upgrade safety report produced by [`Analyzer::analyze_upgrade_patterns`].
#[derive(Debug, Serialize, Clone, Default)]
pub struct UpgradeReport {
    /// Individual findings.
    pub findings: Vec<UpgradeFinding>,
    /// Names of detected upgrade/admin functions.
    pub upgrade_mechanisms: Vec<String>,
    /// Names of detected initialisation functions.
    pub init_functions: Vec<String>,
    /// Names of `#[contracttype]` storage types.
    pub storage_types: Vec<String>,
    /// Actionable suggestions for improving upgrade safety.
    pub suggestions: Vec<String>,
}

impl UpgradeReport {
    /// Returns a default (empty) report.
    pub fn empty() -> Self {
        Self {
            findings: vec![],
            upgrade_mechanisms: vec![],
            init_functions: vec![],
            storage_types: vec![],
            suggestions: vec![],
        }
    }
}

struct UnhandledResultVisitor {
    issues: Vec<UnhandledResultIssue>,
    current_fn: Option<String>,
    is_public_fn: bool,
}

struct UnsafeVisitor {
    patterns: Vec<UnsafePattern>,
}

#[derive(Default)]
struct FunctionSecuritySummary {
    has_mutation: bool,
    has_auth: bool,
    has_external_call: bool,
}

impl FunctionSecuritySummary {
    fn has_sensitive_action(&self) -> bool {
        self.has_mutation || self.has_external_call
    }
}

fn is_reserved_soroban_entrypoint(fn_name: &str) -> bool {
    matches!(fn_name, "__constructor" | "__check_auth")
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method_name = node.method.to_string();
        if method_name == "unwrap" || method_name == "expect" {
            let pattern_type = if method_name == "unwrap" {
                PatternType::Unwrap
            } else {
                PatternType::Expect
            };
            let line = node.span().start().line;
            self.patterns.push(UnsafePattern {
                pattern_type,
                line,
                snippet: quote::quote!(#node).to_string(),
            });
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if node.path.is_ident("panic") {
            let line = node.span().start().line;
            self.patterns.push(UnsafePattern {
                pattern_type: PatternType::Panic,
                line,
                snippet: quote::quote!(#node).to_string(),
            });
        }
        visit::visit_macro(self, node);
    }
}

#[allow(dead_code)]
fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        matches!(&attr.meta, Meta::Path(path) if path.is_ident(name) || path.segments.iter().any(|s| s.ident == name))
    })
}

/// Returns `true` when `attrs` contains `#[cfg(test)]`.
fn is_cfg_test_attrs(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && quote::quote!(#a).to_string().contains("test"))
}

fn is_upgrade_or_admin_fn(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.as_str(),
        "set_admin"
            | "upgrade"
            | "set_authorized"
            | "deploy"
            | "update_admin"
            | "transfer_admin"
            | "change_admin"
    ) || (lower.contains("upgrade") && (lower.contains("contract") || lower.contains("wasm")))
}

fn is_init_fn(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "initialize" || lower == "init" || lower == "initialise"
}

// ── ArithmeticIssue (NEW) ─────────────────────────────────────────────────────

/// Represents an unchecked arithmetic operation that could overflow or underflow.
#[derive(Debug, Serialize, Clone)]
pub struct ArithmeticIssue {
    /// Contract function in which the operation was found.
    pub function_name: String,
    /// The operator: "+", "-", "*", "+=", "-=", "*=".
    pub operation: String,
    /// Human-readable suggestion pointing to the safe alternative.
    pub suggestion: String,
    /// "function_name:line" context string.
    pub location: String,
}

// ── StorageCollisionIssue (NEW) ──────────────────────────────────────────────

/// Represents a potential storage key collision.
#[derive(Debug, Serialize, Clone)]
pub struct StorageCollisionIssue {
    /// The storage key literal or expression.
    pub key_value: String,
    /// The type of the key (e.g. `Symbol`, `BytesN`).
    pub key_type: String,
    /// Source location.
    pub location: String,
    /// Human-readable description.
    pub message: String,
}

/// The kind of event issue detected by [`Analyzer::scan_events`].
#[derive(Debug, Serialize, Clone, PartialEq)]
#[non_exhaustive]
pub enum EventIssueType {
    /// Topics count varies for the same event name.
    InconsistentSchema,
    /// Topic could be optimized with symbol_short!.
    OptimizableTopic,
}

/// An event-related finding (inconsistent schema or optimisable topic).
#[derive(Debug, Serialize, Clone)]
pub struct EventIssue {
    /// Function that emits the event.
    pub function_name: String,
    /// Name of the event.
    pub event_name: String,
    /// Kind of issue.
    pub issue_type: EventIssueType,
    /// Human-readable description.
    pub message: String,
    /// Source location.
    pub location: String,
}

/// A `Result` return value that is silently discarded.
#[derive(Debug, Serialize, Clone)]
pub struct UnhandledResultIssue {
    /// Function containing the unhandled result.
    pub function_name: String,
    /// The call expression that returns `Result`.
    pub call_expression: String,
    /// Human-readable message.
    pub message: String,
    /// Source location.
    pub location: String,
}

// ── Configuration ─────────────────────────────────────────────────────────────
/// Severity level for user-defined custom rules.
pub type RuleSeverity = crate::finding_codes::FindingSeverity;

/// A user-defined regex-based rule loaded from `.sanctify.toml`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomRule {
    /// Display name of the rule.
    pub name: String,
    /// Regex pattern that triggers the rule.
    pub pattern: String,
    /// Severity (defaults to `Warning`).
    #[serde(default)]
    pub severity: RuleSeverity,
}

/// A match produced by a [`CustomRule`].
#[derive(Debug, Serialize, Clone)]
pub struct CustomRuleMatch {
    /// Name of the custom rule that matched.
    pub rule_name: String,
    /// 1-based line number.
    pub line: usize,
    /// Source snippet.
    pub snippet: String,
    /// Severity inherited from the rule.
    pub severity: RuleSeverity,
}

/// Project-level configuration loaded from `.sanctify.toml`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SanctifyConfig {
    /// Paths to skip during directory walking.
    #[serde(default = "default_ignore_paths")]
    pub ignore_paths: Vec<String>,
    /// Names of enabled built-in rules.
    #[serde(default = "default_enabled_rules")]
    pub enabled_rules: Vec<String>,
    /// Ledger-entry size limit in bytes.
    #[serde(default = "default_ledger_limit")]
    pub ledger_limit: usize,
    /// Fraction of `ledger_limit` at which an *approaching* warning fires.
    #[serde(default = "default_approaching_threshold")]
    pub approaching_threshold: f64,
    /// When `true`, use a tighter threshold for size warnings.
    #[serde(default)]
    pub strict_mode: bool,
    /// User-defined regex rules.
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
}

fn default_ignore_paths() -> Vec<String> {
    vec!["target".to_string(), ".git".to_string()]
}

fn default_enabled_rules() -> Vec<String> {
    vec![
        "auth_gaps".to_string(),
        "panics".to_string(),
        "arithmetic".to_string(),
        "ledger_size".to_string(),
        "events".to_string(),
    ]
}

fn default_ledger_limit() -> usize {
    64000
}

fn default_approaching_threshold() -> f64 {
    0.8
}

impl Default for SanctifyConfig {
    fn default() -> Self {
        Self {
            ignore_paths: default_ignore_paths(),
            enabled_rules: default_enabled_rules(),
            ledger_limit: default_ledger_limit(),
            approaching_threshold: default_approaching_threshold(),
            strict_mode: false,
            custom_rules: vec![],
        }
    }
}

fn has_contracttype(attrs: &[syn::Attribute]) -> bool {
    has_attr(attrs, "contracttype")
}

fn classify_size(
    size: usize,
    limit: usize,
    approaching: f64,
    strict: bool,
    strict_threshold: usize,
) -> Option<SizeWarningLevel> {
    if size >= limit || (strict && size >= strict_threshold) {
        Some(SizeWarningLevel::ExceedsLimit)
    } else if size as f64 >= limit as f64 * approaching {
        Some(SizeWarningLevel::ApproachingLimit)
    } else {
        None
    }
}

// ── Analyzer ──────────────────────────────────────────────────────────────────

/// The main analysis engine.
///
/// Create one via [`Analyzer::new`], then call the various `scan_*` /
/// `analyze_*` / `verify_*` methods on contract source code.
pub struct Analyzer {
    /// The active configuration.
    pub config: SanctifyConfig,
    rule_registry: RuleRegistry,
}

impl Analyzer {
    /// Create an [`Analyzer`] with default built-in rules.
    pub fn new(config: SanctifyConfig) -> Self {
        Self {
            config,
            rule_registry: RuleRegistry::default(),
        }
    }

    /// Create an [`Analyzer`] with a custom [`RuleRegistry`].
    pub fn with_rules(config: SanctifyConfig, registry: RuleRegistry) -> Self {
        Self {
            config,
            rule_registry: registry,
        }
    }

    /// Run every registered rule and return all violations.
    pub fn run_rules(&self, source: &str) -> Vec<RuleViolation> {
        self.rule_registry.run_all(source)
    }

    /// Run every registered rule and collect auto-fix patches.
    pub fn run_fixes(&self, source: &str) -> Vec<rules::Patch> {
        self.rule_registry
            .rules
            .iter()
            .flat_map(|rule| rule.fix(source))
            .collect()
    }

    /// Run a single named rule.
    pub fn run_rule(&self, source: &str, name: &str) -> Vec<RuleViolation> {
        self.rule_registry.run_by_name(source, name)
    }

    /// List the names of all registered rules.
    pub fn available_rules(&self) -> Vec<&str> {
        self.rule_registry.available_rules()
    }

    /// Analyse upgrade/admin patterns and return an [`UpgradeReport`].
    pub fn analyze_upgrade_patterns(&self, source: &str) -> UpgradeReport {
        with_panic_guard(|| self.analyze_upgrade_patterns_impl(source))
    }

    /// Verify the contract against the SEP-41 token interface.
    pub fn verify_sep41_interface(&self, source: &str) -> Sep41VerificationReport {
        with_panic_guard(|| sep41::verify(source))
    }

    fn analyze_upgrade_patterns_impl(&self, source: &str) -> UpgradeReport {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return UpgradeReport::empty(),
        };

        let mut report = UpgradeReport::empty();

        for item in &file.items {
            match item {
                Item::Struct(s) => {
                    if has_contracttype(&s.attrs) {
                        report.storage_types.push(s.ident.to_string());
                    }
                }
                Item::Enum(e) => {
                    if has_contracttype(&e.attrs) {
                        report.storage_types.push(e.ident.to_string());
                    }
                }
                Item::Impl(i) => {
                    for impl_item in &i.items {
                        if let syn::ImplItem::Fn(f) = impl_item {
                            if let syn::Visibility::Public(_) = f.vis {
                                let fn_name = f.sig.ident.to_string();
                                if is_init_fn(&fn_name) {
                                    report.init_functions.push(fn_name.clone());
                                }
                                if is_upgrade_or_admin_fn(&fn_name) {
                                    report.upgrade_mechanisms.push(fn_name.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if !report.upgrade_mechanisms.is_empty() {
            report.findings.push(UpgradeFinding {
                category: UpgradeCategory::Governance,
                function_name: report.upgrade_mechanisms.first().cloned(),
                location: report
                    .upgrade_mechanisms
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<unknown>".to_string()),
                message: "Upgrade/admin mechanism detected".to_string(),
                suggestion: "Ensure upgrade/admin functions are properly access-controlled (e.g. require_auth) and consider timelocks/governance.".to_string(),
            });
        }

        report
    }

    /// Detect public functions that mutate storage or call external contracts
    /// without an authentication check.
    pub fn scan_auth_gaps(&self, source: &str) -> Vec<AuthGapIssue> {
        with_panic_guard(|| self.scan_auth_gaps_impl(source))
    }

    /// Run lightweight formal-verification checks via Z3.
    /// Only available when the `smt` feature is enabled (default).
    #[cfg(feature = "smt")]
    pub fn verify_smt_invariants(&self, source: &str) -> Vec<smt::SmtInvariantIssue> {
        with_panic_guard(|| self.verify_smt_invariants_impl(source))
    }

    /// Return no SMT findings when the `smt` feature is disabled.
    #[cfg(not(feature = "smt"))]
    pub fn verify_smt_invariants(&self, _source: &str) -> Vec<smt::SmtInvariantIssue> {
        vec![]
    }

    #[cfg(feature = "smt")]
    fn verify_smt_invariants_impl(&self, source: &str) -> Vec<smt::SmtInvariantIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut issues = Vec::new();

        for item in &file.items {
            match item {
                Item::Impl(i) => {
                    for impl_item in &i.items {
                        if let syn::ImplItem::Fn(f) = impl_item {
                            if !matches!(f.vis, syn::Visibility::Public(_)) {
                                continue;
                            }

                            let mut summary = FunctionSecuritySummary::default();
                            self.check_fn_body(&f.block, &mut summary);

                            if summary.has_external_call {
                                issues.push(smt::SmtInvariantIssue {
                                    function_name: f.sig.ident.to_string(),
                                    description: "Security-property validation is inconclusive across external contract calls; isolate the pure logic or model the callee explicitly before relying on the proof.".to_string(),
                                    location: f.sig.ident.to_string(),
                                });
                            }
                        }
                    }
                }
                Item::Fn(f) => {
                    if !matches!(f.vis, syn::Visibility::Public(_)) {
                        continue;
                    }

                    let mut summary = FunctionSecuritySummary::default();
                    self.check_fn_body(&f.block, &mut summary);

                    if summary.has_external_call {
                        issues.push(smt::SmtInvariantIssue {
                            function_name: f.sig.ident.to_string(),
                            description: "Security-property validation is inconclusive across external contract calls; isolate the pure logic or model the callee explicitly before relying on the proof.".to_string(),
                            location: f.sig.ident.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        issues
    }

    /// Estimate gas / instruction costs per public function.
    pub fn scan_gas_estimation(&self, source: &str) -> Vec<gas_estimator::GasEstimationReport> {
        with_panic_guard(|| self.scan_gas_estimation_impl(source))
    }

    fn scan_gas_estimation_impl(&self, source: &str) -> Vec<gas_estimator::GasEstimationReport> {
        let estimator = gas_estimator::GasEstimator::new();
        estimator.estimate_contract(source)
    }

    fn scan_auth_gaps_impl(&self, source: &str) -> Vec<AuthGapIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut gaps = Vec::new();

        for item in &file.items {
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if let syn::Visibility::Public(_) = f.vis {
                            let fn_name = f.sig.ident.to_string();
                            if is_reserved_soroban_entrypoint(&fn_name) {
                                continue;
                            }
                            let mut summary = FunctionSecuritySummary::default();
                            self.check_fn_body(&f.block, &mut summary);
                            if summary.has_sensitive_action() && !summary.has_auth {
                                gaps.push(AuthGapIssue {
                                    function_name: fn_name,
                                });
                            }
                        }
                    }
                }
            }
        }
        gaps
    }

    // ── Panic / unwrap / expect detection ────────────────────────────────────

    /// Returns all `panic!`, `.unwrap()`, and `.expect()` calls found inside
    /// contract impl functions. Prefer returning `Result` instead.
    pub fn scan_panics(&self, source: &str) -> Vec<PanicIssue> {
        with_panic_guard(|| self.scan_panics_impl(source))
    }

    fn scan_panics_impl(&self, source: &str) -> Vec<PanicIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut issues = Vec::new();
        for item in &file.items {
            match item {
                Item::Impl(i) => {
                    if is_cfg_test_attrs(&i.attrs) {
                        continue;
                    }
                    for impl_item in &i.items {
                        if let syn::ImplItem::Fn(f) = impl_item {
                            if has_attr(&f.attrs, "test") {
                                continue;
                            }
                            let fn_name = f.sig.ident.to_string();
                            self.check_fn_panics(&f.block, &fn_name, &mut issues);
                        }
                    }
                }
                Item::Mod(m) => {
                    if is_cfg_test_attrs(&m.attrs) {
                        continue;
                    }
                    if let Some((_, items)) = &m.content {
                        for item in items {
                            if let Item::Fn(f) = item {
                                if has_attr(&f.attrs, "test") {
                                    continue;
                                }
                                let fn_name = f.sig.ident.to_string();
                                self.check_fn_panics(&f.block, &fn_name, &mut issues);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        issues
    }

    fn check_fn_panics(&self, block: &syn::Block, fn_name: &str, issues: &mut Vec<PanicIssue>) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => self.check_expr_panics(expr, fn_name, issues),
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.check_expr_panics(&init.expr, fn_name, issues);
                    }
                }
                // In syn 2.0, bare macro calls (e.g. `panic!(...)`) are Stmt::Macro,
                // not Stmt::Expr(Expr::Macro(...)).
                syn::Stmt::Macro(m) => {
                    if m.mac.path.is_ident("panic") {
                        issues.push(PanicIssue {
                            function_name: fn_name.to_string(),
                            issue_type: "panic!".to_string(),
                            location: fn_name.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn check_expr_panics(&self, expr: &syn::Expr, fn_name: &str, issues: &mut Vec<PanicIssue>) {
        match expr {
            syn::Expr::Macro(m) => {
                if m.mac.path.is_ident("panic") {
                    issues.push(PanicIssue {
                        function_name: fn_name.to_string(),
                        issue_type: "panic!".to_string(),
                        location: fn_name.to_string(),
                    });
                }
            }
            syn::Expr::MethodCall(m) => {
                let method_name = m.method.to_string();
                if method_name == "unwrap" || method_name == "expect" {
                    issues.push(PanicIssue {
                        function_name: fn_name.to_string(),
                        issue_type: method_name,
                        location: fn_name.to_string(),
                    });
                }
                self.check_expr_panics(&m.receiver, fn_name, issues);
                for arg in &m.args {
                    self.check_expr_panics(arg, fn_name, issues);
                }
            }
            syn::Expr::Call(c) => {
                for arg in &c.args {
                    self.check_expr_panics(arg, fn_name, issues);
                }
            }
            syn::Expr::Block(b) => self.check_fn_panics(&b.block, fn_name, issues),
            syn::Expr::If(i) => {
                self.check_expr_panics(&i.cond, fn_name, issues);
                self.check_fn_panics(&i.then_branch, fn_name, issues);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr_panics(else_expr, fn_name, issues);
                }
            }
            syn::Expr::Match(m) => {
                self.check_expr_panics(&m.expr, fn_name, issues);
                for arm in &m.arms {
                    self.check_expr_panics(&arm.body, fn_name, issues);
                }
            }
            _ => {}
        }
    }

    // ── Mutation / auth helpers ───────────────────────────────────────────────

    fn check_fn_body(&self, block: &syn::Block, summary: &mut FunctionSecuritySummary) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => self.check_expr(expr, summary),
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.check_expr(&init.expr, summary);
                    }
                }
                syn::Stmt::Macro(m) => {
                    if m.mac.path.is_ident("require_auth")
                        || m.mac.path.is_ident("require_auth_for_args")
                    {
                        summary.has_auth = true;
                    }
                }
                _ => {}
            }
        }
    }

    fn check_expr(&self, expr: &syn::Expr, summary: &mut FunctionSecuritySummary) {
        match expr {
            syn::Expr::Call(c) => {
                if let syn::Expr::Path(p) = &*c.func {
                    if let Some(segment) = p.path.segments.last() {
                        let ident = segment.ident.to_string();
                        if ident == "require_auth" || ident == "require_auth_for_args" {
                            summary.has_auth = true;
                        }
                    }
                }
                for arg in &c.args {
                    self.check_expr(arg, summary);
                }
            }
            syn::Expr::MethodCall(m) => {
                let method_name = m.method.to_string();
                if method_name == "set" || method_name == "update" || method_name == "remove" {
                    // Heuristic: check if receiver chain contains "storage"
                    let receiver_str = quote::quote!(#m.receiver).to_string();
                    if receiver_str.contains("storage")
                        || receiver_str.contains("persistent")
                        || receiver_str.contains("temporary")
                        || receiver_str.contains("instance")
                    {
                        summary.has_mutation = true;
                    }
                }
                if method_name == "require_auth" || method_name == "require_auth_for_args" {
                    summary.has_auth = true;
                }
                if is_external_contract_method_call(m) {
                    summary.has_external_call = true;
                }
                self.check_expr(&m.receiver, summary);
                for arg in &m.args {
                    self.check_expr(arg, summary);
                }
            }
            syn::Expr::Block(b) => self.check_fn_body(&b.block, summary),
            syn::Expr::If(i) => {
                self.check_expr(&i.cond, summary);
                self.check_fn_body(&i.then_branch, summary);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr(else_expr, summary);
                }
            }
            syn::Expr::Match(m) => {
                self.check_expr(&m.expr, summary);
                for arm in &m.arms {
                    self.check_expr(&arm.body, summary);
                }
            }
            _ => {}
        }
    }

    // ── Storage collision (stub) ──────────────────────────────────────────────

    /// (Stub) Check a set of storage keys for collisions.
    pub fn check_storage_collisions(&self, _keys: Vec<String>) -> bool {
        false
    }

    // ── Ledger size analysis ──────────────────────────────────────────────────

    /// Analyzes `#[contracttype]` structs and enums, estimates serialized size,
    /// and warns when approaching or exceeding the ledger entry limit (e.g. 64KB).
    pub fn analyze_ledger_size(&self, source: &str) -> Vec<SizeWarning> {
        with_panic_guard(|| self.analyze_ledger_size_impl(source))
    }

    fn analyze_ledger_size_impl(&self, source: &str) -> Vec<SizeWarning> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let limit = self.config.ledger_limit;
        let approaching = self.config.approaching_threshold;
        let strict = self.config.strict_mode;
        // Use 90% as strict threshold heuristic
        let strict_threshold = (limit as f64 * DEFAULT_STRICT_THRESHOLD) as usize;

        let mut warnings = Vec::new();

        for item in &file.items {
            match item {
                Item::Struct(s) => {
                    if has_contracttype(&s.attrs) {
                        let size = self.estimate_struct_size(s);
                        if let Some(level) =
                            classify_size(size, limit, approaching, strict, strict_threshold)
                        {
                            warnings.push(SizeWarning {
                                struct_name: s.ident.to_string(),
                                estimated_size: size,
                                limit,
                                level,
                            });
                        }
                    }
                }
                Item::Enum(e) => {
                    if has_contracttype(&e.attrs) {
                        let size = self.estimate_enum_size(e);
                        if let Some(level) =
                            classify_size(size, limit, approaching, strict, strict_threshold)
                        {
                            warnings.push(SizeWarning {
                                struct_name: e.ident.to_string(),
                                estimated_size: size,
                                limit,
                                level,
                            });
                        }
                    }
                }
                Item::Impl(_) | Item::Macro(_) => {}
                _ => {}
            }
        }

        warnings
    }

    // ── Event Consistency and Optimization ──────────────────────────────────────

    fn extract_topics(line: &str) -> String {
        if let Some(start_paren) = line.find('(') {
            let after_publish = &line[start_paren + 1..];
            if let Some(end_paren) = after_publish.rfind(')') {
                let topics_content = &after_publish[..end_paren];
                if topics_content.contains(',') || topics_content.starts_with('(') {
                    return topics_content.to_string();
                }
            }
        }
        if let Some(vec_start) = line.find("vec![") {
            let after_vec = &line[vec_start + 5..];
            if let Some(end_bracket) = after_vec.find(']') {
                return after_vec[..end_bracket].to_string();
            }
        }
        String::new()
    }

    fn extract_event_name(line: &str) -> Option<String> {
        if let Some(start) = line.find('(') {
            let content = &line[start..];
            if let Some(name_end) = content.find(',') {
                let name_part = &content[1..name_end];
                let clean_name = name_part.trim().trim_matches('"');
                if !clean_name.is_empty() {
                    return Some(clean_name.to_string());
                }
            } else if let Some(end_paren) = content.find(')') {
                let name_part = &content[1..end_paren];
                let clean_name = name_part.trim().trim_matches('"');
                if !clean_name.is_empty() {
                    return Some(clean_name.to_string());
                }
            }
        }
        None
    }

    /// Scans for `env.events().publish(topics, data)` and checks:
    /// 1. Consistency of topic counts for the same event name.
    /// 2. Opportunities to use `symbol_short!` for gas savings.
    pub fn scan_events(&self, source: &str) -> Vec<EventIssue> {
        with_panic_guard(|| self.scan_events_impl(source))
    }

    fn scan_events_impl(&self, source: &str) -> Vec<EventIssue> {
        let mut issues = Vec::new();
        let mut event_schemas: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        let mut issue_locations: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();

            if line.contains("env.events().publish(") || line.contains("env.events().emit(") {
                let topics_str = Self::extract_topics(line);
                let topic_count = if topics_str.is_empty() {
                    0
                } else {
                    topics_str.matches(',').count() + 1
                };

                let event_name = Self::extract_event_name(line)
                    .unwrap_or_else(|| format!("unknown_{}", line_num));

                let location = format!("line {}", line_num + 1);
                let _location_key = format!("{}:{}", event_name, topic_count);

                if let Some(previous_counts) = event_schemas.get(&event_name) {
                    for &prev_count in previous_counts {
                        if prev_count != topic_count {
                            let issue_key = format!("{}:{}:inconsistent", event_name, line_num + 1);
                            if !issue_locations.contains(&issue_key) {
                                issue_locations.insert(issue_key);
                                issues.push(EventIssue {
                                    function_name: "unknown".to_string(), // scan_events_impl is regex-based, function context is limited
                                    event_name: event_name.clone(),
                                    issue_type: EventIssueType::InconsistentSchema,
                                    message: format!(
                                        "Event '{}' has inconsistent topic count. Previous: {}, Current: {}",
                                        event_name, prev_count, topic_count
                                    ),
                                    location: location.clone(),
                                });
                            }
                        }
                    }
                }

                event_schemas
                    .entry(event_name.clone())
                    .or_default()
                    .push(topic_count);

                if !line.contains("symbol_short!") && topic_count > 0 {
                    let has_string_topic = line.contains("\"") || line.contains("String");
                    if has_string_topic {
                        let issue_key = format!("{}:{}:gas_optimization", event_name, line_num + 1);
                        if !issue_locations.contains(&issue_key) {
                            issue_locations.insert(issue_key);
                            issues.push(EventIssue {
                                function_name: "unknown".to_string(),
                                event_name,
                                issue_type: EventIssueType::OptimizableTopic,
                                message: "Consider using symbol_short! for short topic names to save gas.".to_string(),
                                location: format!("line {}", line_num + 1),
                            });
                        }
                    }
                }
            }
        }

        issues
    }

    // ── Unsafe-pattern visitor ────────────────────────────────────────────────

    /// Visitor-based scan for `panic!`, `.unwrap()`, `.expect()` with line
    /// numbers derived from proc-macro2 span locations.
    pub fn analyze_unsafe_patterns(&self, source: &str) -> Vec<UnsafePattern> {
        with_panic_guard(|| self.analyze_unsafe_patterns_impl(source))
    }

    fn analyze_unsafe_patterns_impl(&self, source: &str) -> Vec<UnsafePattern> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = UnsafeVisitor {
            patterns: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.patterns
    }

    // ── Arithmetic overflow detection (NEW) ───────────────────────────────────

    /// Scans contract impl functions for unchecked arithmetic (`+`, `-`, `*`,
    /// `+=`, `-=`, `*=`) and suggests the corresponding `checked_*` or
    /// `saturating_*` alternatives.
    ///
    /// One issue is reported per (function, operator) pair to keep output
    /// actionable. Line numbers are included when span-location info is
    /// available (requires `proc-macro2` with `span-locations` feature).
    pub fn scan_arithmetic_overflow(&self, source: &str) -> Vec<ArithmeticIssue> {
        with_panic_guard(|| self.scan_arithmetic_overflow_impl(source))
    }

    fn scan_arithmetic_overflow_impl(&self, source: &str) -> Vec<ArithmeticIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = ArithVisitor {
            issues: Vec::new(),
            current_fn: None,
            seen: HashSet::new(),
            index_depth: 0,
            test_mod_depth: 0,
        };
        visitor.visit_file(&file);
        visitor.issues
    }

    /// Run regex-based custom rules from config. Returns matches with line and snippet.
    pub fn analyze_custom_rules(&self, source: &str, rules: &[CustomRule]) -> Vec<CustomRuleMatch> {
        use regex::Regex;

        let mut matches = Vec::new();
        for rule in rules {
            let re = match Regex::new(&rule.pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for (line_no, line) in source.lines().enumerate() {
                let line_num = line_no + 1;
                if re.find(line).is_some() {
                    matches.push(CustomRuleMatch {
                        rule_name: rule.name.clone(),
                        line: line_num,
                        snippet: line.trim().to_string(),
                        severity: rule.severity,
                    });
                }
            }
        }
        matches
    }

    /// Extract cross-contract call edges for the call-graph.
    pub fn scan_invoke_contract_calls(
        &self,
        source: &str,
        caller: &str,
        file_path: &str,
    ) -> Vec<ContractCallEdge> {
        with_panic_guard(|| self.scan_invoke_contract_calls_impl(source, caller, file_path))
    }

    fn scan_invoke_contract_calls_impl(
        &self,
        source: &str,
        caller: &str,
        file_path: &str,
    ) -> Vec<ContractCallEdge> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = InvokeContractVisitor {
            edges: Vec::new(),
            caller: caller.to_string(),
            file_path: file_path.to_string(),
        };
        visitor.visit_file(&file);
        visitor.edges
    }

    // ── Storage key collision detection (NEW) ─────────────────────────────────

    /// Scans for potential storage key collisions by analyzing constants,
    /// Symbol::new calls, and symbol_short! macros.
    /// Detect potential storage-key collisions within a single source file.
    pub fn scan_storage_collisions(&self, source: &str) -> Vec<StorageCollisionIssue> {
        with_panic_guard(|| self.scan_storage_collisions_impl(source))
    }

    fn scan_storage_collisions_impl(&self, source: &str) -> Vec<StorageCollisionIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = storage_collision::StorageVisitor::new();
        visitor.visit_file(&file);
        visitor.final_check();
        visitor.collisions
    }

    /// Detect `Result` return values that are silently discarded.
    pub fn scan_unhandled_results(&self, source: &str) -> Vec<UnhandledResultIssue> {
        with_panic_guard(|| self.scan_unhandled_results_impl(source))
    }

    fn scan_unhandled_results_impl(&self, source: &str) -> Vec<UnhandledResultIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = UnhandledResultVisitor {
            issues: Vec::new(),
            current_fn: None,
            is_public_fn: false,
        };
        visitor.visit_file(&file);
        visitor.issues
    }

    // ── Size estimation helpers ───────────────────────────────────────────────

    fn estimate_enum_size(&self, e: &syn::ItemEnum) -> usize {
        const DISCRIMINANT_SIZE: usize = 4;
        let mut max_variant = 0usize;
        for v in &e.variants {
            let mut variant_size = 0;
            match &v.fields {
                syn::Fields::Named(fields) => {
                    for f in &fields.named {
                        variant_size += self.estimate_type_size(&f.ty);
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    for f in &fields.unnamed {
                        variant_size += self.estimate_type_size(&f.ty);
                    }
                }
                syn::Fields::Unit => {}
            }
            max_variant = max_variant.max(variant_size);
        }
        DISCRIMINANT_SIZE + max_variant
    }

    fn estimate_struct_size(&self, s: &syn::ItemStruct) -> usize {
        let mut total = 0;
        match &s.fields {
            Fields::Named(fields) => {
                for f in &fields.named {
                    total += self.estimate_type_size(&f.ty);
                }
            }
            Fields::Unnamed(fields) => {
                for f in &fields.unnamed {
                    total += self.estimate_type_size(&f.ty);
                }
            }
            Fields::Unit => {}
        }
        total
    }

    fn estimate_type_size(&self, ty: &Type) -> usize {
        match ty {
            Type::Path(tp) => {
                if let Some(seg) = tp.path.segments.last() {
                    let base = match seg.ident.to_string().as_str() {
                        "u32" | "i32" | "bool" => 4,
                        "u64" | "i64" => 8,
                        "u128" | "i128" | "I128" | "U128" => 16,
                        "Address" => 32,
                        "Bytes" | "BytesN" | "String" | "Symbol" => 64,
                        "Vec" => {
                            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return 8 + self.estimate_type_size(inner);
                                }
                            }
                            128
                        }
                        "Map" => {
                            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                let inner: usize = args
                                    .args
                                    .iter()
                                    .filter_map(|a| {
                                        if let syn::GenericArgument::Type(t) = a {
                                            Some(self.estimate_type_size(t))
                                        } else {
                                            None
                                        }
                                    })
                                    .sum();
                                if inner > 0 {
                                    return 16 + inner * 2;
                                }
                            }
                            128
                        }
                        "Option" => {
                            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return 1 + self.estimate_type_size(inner);
                                }
                            }
                            32
                        }
                        _ => 32,
                    };
                    base
                } else {
                    8
                }
            }
            Type::Array(arr) => {
                if let syn::Expr::Lit(expr_lit) = &arr.len {
                    if let syn::Lit::Int(lit) = &expr_lit.lit {
                        if let Ok(n) = lit.base10_parse::<usize>() {
                            return n * self.estimate_type_size(&arr.elem);
                        }
                    }
                }
                64
            }
            _ => 8,
        }
    }
}

/// An edge in the cross-contract call graph.
#[derive(Debug, Clone, Serialize)]
pub struct ContractCallEdge {
    /// The calling contract.
    pub caller: String,
    /// The callee contract (derived from the expression).
    pub callee: String,
    /// File containing the call.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// Raw expression used as the contract-id argument.
    pub contract_id_expr: String,
    /// Optional function-name expression.
    pub function_expr: Option<String>,
}

/// Render a set of [`ContractCallEdge`]s as a Graphviz DOT string.
pub fn callgraph_to_dot(edges: &[ContractCallEdge]) -> String {
    use std::collections::BTreeMap;

    let mut per_pair: BTreeMap<(String, String), Vec<&ContractCallEdge>> = BTreeMap::new();
    for e in edges {
        per_pair
            .entry((e.caller.clone(), e.callee.clone()))
            .or_default()
            .push(e);
    }

    let mut out = String::new();
    out.push_str("digraph ContractCallGraph {\n");
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [shape=box, fontname=\"Helvetica\"];\n");

    for ((caller, callee), calls) in per_pair {
        let mut label = String::new();
        for (idx, c) in calls.iter().enumerate() {
            if idx > 0 {
                label.push_str("\\n");
            }
            label.push_str(&format!("{}:{}", c.file, c.line));
            if let Some(fx) = &c.function_expr {
                label.push_str(&format!(" [{}]", fx));
            }
        }

        out.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
            escape_dot(&caller),
            escape_dot(&callee),
            escape_dot(&label)
        ));
    }

    out.push_str("}\n");
    out
}

fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

struct InvokeContractVisitor {
    edges: Vec<ContractCallEdge>,
    caller: String,
    file_path: String,
}

impl<'ast> Visit<'ast> for InvokeContractVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "invoke_contract" {
            let contract_id_expr = node
                .args
                .first()
                .map(|e| quote::quote!(#e).to_string())
                .unwrap_or_else(|| "<missing>".to_string());

            let function_expr = node
                .args
                .iter()
                .nth(1)
                .map(|e| quote::quote!(#e).to_string());

            let callee = simplify_expr_string(&contract_id_expr);
            let line = node.span().start().line;

            self.edges.push(ContractCallEdge {
                caller: self.caller.clone(),
                callee,
                file: self.file_path.clone(),
                line,
                contract_id_expr,
                function_expr: function_expr.map(|s| simplify_expr_string(&s)),
            });
        }

        visit::visit_expr_method_call(self, node);
    }
}

fn simplify_expr_string(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_external_contract_method_call(method_call: &syn::ExprMethodCall) -> bool {
    if method_call.method == "invoke_contract" {
        return true;
    }

    receiver_looks_like_external_client(&method_call.receiver)
        && !method_looks_read_only(&method_call.method.to_string())
}

fn receiver_looks_like_external_client(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Call(call) => {
            if let syn::Expr::Path(path) = &*call.func {
                return path_looks_like_client_constructor(&path.path);
            }
            false
        }
        syn::Expr::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| ident_looks_like_client(&segment.ident.to_string()))
            .unwrap_or(false),
        syn::Expr::Reference(reference) => receiver_looks_like_external_client(&reference.expr),
        syn::Expr::Paren(paren) => receiver_looks_like_external_client(&paren.expr),
        syn::Expr::Group(group) => receiver_looks_like_external_client(&group.expr),
        _ => false,
    }
}

fn path_looks_like_client_constructor(path: &syn::Path) -> bool {
    let mut saw_client_type = false;

    for segment in &path.segments {
        let ident = segment.ident.to_string();
        if ident_looks_like_client(&ident) {
            saw_client_type = true;
        }

        if ident == "new" && saw_client_type {
            return true;
        }
    }

    false
}

fn ident_looks_like_client(ident: &str) -> bool {
    let lower = ident.to_lowercase();
    lower.ends_with("client") || lower.ends_with("_client")
}

fn method_looks_read_only(method_name: &str) -> bool {
    matches!(
        method_name,
        "balance" | "paused" | "allowance" | "decimals" | "name" | "symbol"
    ) || method_name.starts_with("get_")
        || method_name.starts_with("is_")
        || method_name.starts_with("has_")
}

// ── EventVisitor (stubs/helpers moved) ──────────────────────────────────────

impl<'ast> Visit<'ast> for UnhandledResultVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev_fn = self.current_fn.take();
        let prev_public = self.is_public_fn;

        self.current_fn = Some(node.sig.ident.to_string());
        self.is_public_fn = matches!(node.vis, syn::Visibility::Public(_));

        let fn_returns_result = Self::is_result_returning_fn(&node.sig);

        for stmt in &node.block.stmts {
            self.check_statement_for_unhandled_result(stmt, fn_returns_result);
        }

        self.current_fn = prev_fn;
        self.is_public_fn = prev_public;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev_fn = self.current_fn.take();
        let prev_public = self.is_public_fn;

        self.current_fn = Some(node.sig.ident.to_string());
        self.is_public_fn = matches!(node.vis, syn::Visibility::Public(_));

        let fn_returns_result = Self::is_result_returning_fn(&node.sig);

        for stmt in &node.block.stmts {
            self.check_statement_for_unhandled_result(stmt, fn_returns_result);
        }

        self.current_fn = prev_fn;
        self.is_public_fn = prev_public;
    }
}

impl UnhandledResultVisitor {
    fn check_statement_for_unhandled_result(&mut self, stmt: &syn::Stmt, fn_returns_result: bool) {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.check_expr_for_unhandled_result(expr, fn_returns_result);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.check_expr_for_unhandled_result(&init.expr, fn_returns_result);
                }
            }
            syn::Stmt::Macro(_) => {}
            _ => {}
        }
    }

    fn check_expr_for_unhandled_result(&mut self, expr: &syn::Expr, fn_returns_result: bool) {
        match expr {
            syn::Expr::Call(call) => {
                if Self::is_handled(expr) {
                    return;
                }
                if Self::call_returns_result(call) && !fn_returns_result && self.is_public_fn {
                    if let Some(fn_name) = &self.current_fn {
                        let line = expr.span().start().line;
                        self.issues.push(UnhandledResultIssue {
                        function_name: fn_name.to_string(),
                        call_expression: Self::expr_to_string(expr),
                        message: "Result returned from function call is not handled. Use ?, match, or .unwrap()/.expect() to handle the Result.".to_string(),
                        location: format!("{}:{}", fn_name, line),
                    });
                    }
                }
                for arg in &call.args {
                    self.check_expr_for_unhandled_result(arg, fn_returns_result);
                }
            }
            syn::Expr::MethodCall(m) => {
                if Self::is_handled(expr) {
                    return;
                }
                self.check_expr_for_unhandled_result(&m.receiver, fn_returns_result);
                for arg in &m.args {
                    self.check_expr_for_unhandled_result(arg, fn_returns_result);
                }
            }
            syn::Expr::Try(e) => {
                self.check_expr_for_unhandled_result(&e.expr, true);
            }
            syn::Expr::Match(m) => {
                for arm in &m.arms {
                    self.check_expr_for_unhandled_result(&arm.body, fn_returns_result);
                }
            }
            syn::Expr::If(i) => {
                self.check_expr_for_unhandled_result(&i.cond, fn_returns_result);
                self.check_block_for_unhandled_result(&i.then_branch, fn_returns_result);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr_for_unhandled_result(else_expr, fn_returns_result);
                }
            }
            syn::Expr::Block(b) => {
                self.check_block_for_unhandled_result(&b.block, fn_returns_result);
            }
            syn::Expr::Closure(c) => {
                self.check_expr_for_unhandled_result(&c.body, fn_returns_result);
            }
            syn::Expr::Assign(a) => {
                self.check_expr_for_unhandled_result(&a.right, fn_returns_result);
            }
            syn::Expr::Binary(b) => {
                self.check_expr_for_unhandled_result(&b.left, fn_returns_result);
                self.check_expr_for_unhandled_result(&b.right, fn_returns_result);
            }
            syn::Expr::Tuple(t) => {
                for elem in &t.elems {
                    self.check_expr_for_unhandled_result(elem, fn_returns_result);
                }
            }
            syn::Expr::Array(a) => {
                for elem in &a.elems {
                    self.check_expr_for_unhandled_result(elem, fn_returns_result);
                }
            }
            syn::Expr::Struct(s) => {
                for field in &s.fields {
                    self.check_expr_for_unhandled_result(&field.expr, fn_returns_result);
                }
            }
            syn::Expr::Return(r) => {
                if let Some(expr) = &r.expr {
                    self.check_expr_for_unhandled_result(expr, true);
                }
            }
            syn::Expr::Path(_) => {}
            syn::Expr::Lit(_) => {}
            syn::Expr::Field(f) => {
                self.check_expr_for_unhandled_result(&f.base, fn_returns_result);
            }
            syn::Expr::Index(i) => {
                self.check_expr_for_unhandled_result(&i.expr, fn_returns_result);
                self.check_expr_for_unhandled_result(&i.index, fn_returns_result);
            }
            syn::Expr::Reference(r) => {
                self.check_expr_for_unhandled_result(&r.expr, fn_returns_result);
            }
            syn::Expr::Unary(u) => {
                self.check_expr_for_unhandled_result(&u.expr, fn_returns_result);
            }
            syn::Expr::Cast(c) => {
                self.check_expr_for_unhandled_result(&c.expr, fn_returns_result);
            }
            _ => {}
        }
    }

    fn check_block_for_unhandled_result(&mut self, block: &syn::Block, fn_returns_result: bool) {
        for stmt in &block.stmts {
            self.check_statement_for_unhandled_result(stmt, fn_returns_result);
        }
    }

    fn call_returns_result(call: &syn::ExprCall) -> bool {
        if let syn::Expr::Path(p) = &*call.func {
            if let Some(seg) = p.path.segments.last() {
                let name = seg.ident.to_string();
                if matches!(name.as_str(), "Ok" | "Err" | "Some" | "None" | "panic") {
                    return false;
                }
                if matches!(
                    name.as_str(),
                    "assert"
                        | "assert_eq"
                        | "assert_ne"
                        | "debug_assert"
                        | "debug_assert_eq"
                        | "debug_assert_ne"
                        | "println"
                        | "print"
                        | "eprintln"
                        | "eprint"
                        | "format"
                        | "vec"
                        | "panic"
                ) {
                    return false;
                }
                return true;
            }
        }
        false
    }

    fn is_result_returning_fn(sig: &syn::Signature) -> bool {
        if let syn::ReturnType::Type(_, ty) = &sig.output {
            if let syn::Type::Path(tp) = &**ty {
                if let Some(seg) = tp.path.segments.last() {
                    return seg.ident == "Result";
                }
            }
        }
        false
    }

    fn is_handled(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Try(_) => true,
            syn::Expr::Match(_) => true,
            syn::Expr::MethodCall(m) => {
                let method = m.method.to_string();
                matches!(
                    method.as_str(),
                    "unwrap"
                        | "expect"
                        | "unwrap_or"
                        | "unwrap_or_else"
                        | "unwrap_or_default"
                        | "ok"
                        | "err"
                        | "is_ok"
                        | "is_err"
                        | "map"
                        | "map_err"
                        | "and_then"
                        | "or_else"
                        | "unwrap_unchecked"
                        | "expect_unchecked"
                        | "as_ref"
                        | "as_mut"
                        | "clone"
                        | "inspect"
                        | "inspect_err"
                )
            }
            syn::Expr::Assign(a) => Self::is_handled(&a.right),
            syn::Expr::Call(c) => {
                if let syn::Expr::Path(p) = &*c.func {
                    if let Some(seg) = p.path.segments.last() {
                        if seg.ident == "Ok" || seg.ident == "Err" {
                            return true;
                        }
                    }
                }
                false
            }
            syn::Expr::Block(_) => true,
            _ => false,
        }
    }

    fn expr_to_string(expr: &syn::Expr) -> String {
        let s = quote::quote!(#expr).to_string();
        if s.len() > 80 {
            format!("{}...", &s[..77])
        } else {
            s
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_with_macros() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env};

            #[contract]
            pub struct MyContract;

            #[contractimpl]
            impl MyContract {
                pub fn hello(env: Env) {}
            }

            #[contracttype]
            pub struct SmallData {
                pub x: u32,
            }

            #[contracttype]
            pub struct BigData {
                pub buffer: Bytes,
                pub large: u128,
            }
        "#;
        let warnings = analyzer.analyze_ledger_size(source);
        // SmallData: 4 bytes — BigData: 64 + 16 = 80 bytes — both under 64 KB
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_analyze_with_limit() {
        let config = SanctifyConfig {
            ledger_limit: 50,
            ..Default::default()
        };
        let analyzer = Analyzer::new(config);
        let source = r#"
            #[contracttype]
            pub struct ExceedsLimit {
                pub buffer: Bytes, // 64 bytes estimated
            }
        "#;
        let warnings = analyzer.analyze_ledger_size(source);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].struct_name, "ExceedsLimit");
        assert_eq!(warnings[0].estimated_size, 64);
        assert_eq!(warnings[0].level, SizeWarningLevel::ExceedsLimit);
    }

    /*
        #[test]
        fn test_ledger_size_enum_and_approaching() {
            let mut config = SanctifyConfig::default();
            config.ledger_limit = 100;
            config.approaching_threshold = 0.5;
            let analyzer = Analyzer::new(config);
            let source = r#"
                #[contracttype]
                pub enum DataKey {
                    Balance(Address),
                    Admin,
                }

                #[contracttype]
                pub struct NearLimit {
                    pub a: u128,
                    pub b: u128,
                    pub c: u128,
                    pub d: u128,
                }
            "#;
            let warnings = analyzer.analyze_ledger_size(source);
            assert!(warnings.iter().any(|w| w.struct_name == "NearLimit"), "NearLimit (64 bytes) should exceed 50% of 100");
            assert!(warnings.iter().any(|w| w.level == SizeWarningLevel::ApproachingLimit));
        }
    */

    #[test]
    fn test_complex_macro_no_panic() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            macro_rules! complex {
                ($($t:tt)*) => { $($t)* };
            }

            complex! {
                pub struct MyStruct {
                    pub x: u32,
                }
            }

            #[contractimpl]
            impl Contract {
                pub fn test() {
                    let x = symbol_short!("test");
                }
            }
        "#;
        let _ = analyzer.analyze_ledger_size(source);
    }

    #[test]
    fn test_heavy_macro_usage_graceful() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env};

            #[contract]
            pub struct Token;

            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    // Heavy macro expansion - analyzer must not panic
                }
            }
        "#;
        let _ = analyzer.scan_auth_gaps(source);
        let _ = analyzer.scan_panics(source);
        let _ = analyzer.analyze_unsafe_patterns(source);
        let _ = analyzer.analyze_ledger_size(source);
        let _ = analyzer.scan_arithmetic_overflow(source);
    }

    #[test]
    fn test_scan_auth_gaps() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn set_data(env: Env, val: u32) {
                    env.storage().instance().set(&DataKey::Val, &val);
                }

                pub fn set_data_secure(env: Env, val: u32) {
                    env.require_auth();
                    env.storage().instance().set(&DataKey::Val, &val);
                }

                pub fn get_data(env: Env) -> u32 {
                    env.storage().instance().get(&DataKey::Val).unwrap_or(0)
                }

                pub fn no_storage(env: Env) {
                    let x = 1 + 1;
                }
            }
        "#;
        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].function_name, "set_data");
    }

    #[test]
    fn test_scan_auth_gaps_flags_read_modify_write_without_auth() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    let from_balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
                    let to_balance: i128 = env.storage().persistent().get(&to).unwrap_or(0);

                    env.storage().persistent().set(&from, &(from_balance - amount));
                    env.storage().persistent().set(&to, &(to_balance + amount));
                }

                pub fn transfer_secure(env: Env, from: Address, to: Address, amount: i128) {
                    from.require_auth();

                    let from_balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
                    let to_balance: i128 = env.storage().persistent().get(&to).unwrap_or(0);

                    env.storage().persistent().set(&from, &(from_balance - amount));
                    env.storage().persistent().set(&to, &(to_balance + amount));
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].function_name, "transfer");
    }

    #[test]
    fn test_scan_auth_gaps_flags_external_contract_calls_without_auth() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Router {
                pub fn forward(env: Env, target: Address, to: Address, amount: i128) {
                    let fn_name = Symbol::new(&env, "transfer");
                    env.invoke_contract::<()>(&target, &fn_name, (&to, &amount));
                }

                pub fn forward_secure(env: Env, target: Address, admin: Address, to: Address, amount: i128) {
                    admin.require_auth();
                    let fn_name = Symbol::new(&env, "transfer");
                    env.invoke_contract::<()>(&target, &fn_name, (&to, &amount));
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].function_name, "forward");
    }

    #[test]
    fn test_scan_auth_gaps_ignores_reserved_soroban_entrypoints() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl AccountContract {
                pub fn __constructor(env: Env, admin: Address) {
                    env.storage().instance().set(&symbol_short!("admin"), &admin);
                }

                pub fn reset_admin(env: Env, admin: Address) {
                    env.storage().instance().set(&symbol_short!("admin"), &admin);
                }
            }

            #[contractimpl(contracttrait)]
            impl CustomAccountInterface for AccountContract {
                pub fn __check_auth(env: Env, payload: BytesN<32>) {
                    env.storage().temporary().set(&payload, &true);
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].function_name, "reset_admin");
    }

    #[test]
    fn test_scan_auth_gaps_ignores_read_only_external_client_calls() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Pool {
                pub fn get_balance(env: Env, token: Address) -> i128 {
                    let client = token::Client::new(&env, &token);
                    client.balance(&env.current_contract_address())
                }

                pub fn forward_transfer(env: Env, token: Address, to: Address, amount: i128) {
                    let client = token::Client::new(&env, &token);
                    client.transfer(&env.current_contract_address(), &to, &amount);
                }
            }
        "#;

        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].function_name, "forward_transfer");
    }

    #[test]
    #[cfg(feature = "smt")]
    fn test_verify_smt_invariants_reports_external_contract_boundaries() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Settlement {
                pub fn settle(env: Env, market: Address, amount: i128) {
                    let fn_name = Symbol::new(&env, "settle_position");
                    env.invoke_contract::<()>(&market, &fn_name, (&amount,));
                }

                pub fn local_only(env: Env, amount: i128) {
                    env.storage().instance().set(&symbol_short!("amount"), &amount);
                }
            }
        "#;

        let issues = analyzer.verify_smt_invariants(source);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].function_name, "settle");
        assert!(
            issues[0].description.contains("external contract calls"),
            "expected external-call proof boundary warning"
        );
    }

    #[test]
    fn test_scan_panics() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn unsafe_fn(env: Env) {
                    panic!("Something went wrong");
                }

                pub fn unsafe_unwrap(env: Env) {
                    let x: Option<u32> = None;
                    let y = x.unwrap();
                }

                pub fn unsafe_expect(env: Env) {
                    let x: Option<u32> = None;
                    let y = x.expect("Failed to get x");
                }

                pub fn safe_fn(env: Env) -> Result<(), u32> {
                    Ok(())
                }
            }
        "#;
        let issues = analyzer.scan_panics(source);
        assert_eq!(issues.len(), 3);

        let types: Vec<String> = issues.iter().map(|i| i.issue_type.clone()).collect();
        assert!(types.contains(&"panic!".to_string()));
        assert!(types.contains(&"unwrap".to_string()));
        assert!(types.contains(&"expect".to_string()));
    }

    // ── Arithmetic overflow tests ─────────────────────────────────────────────

    #[test]
    fn test_scan_arithmetic_overflow_basic() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn add_balances(env: Env, a: u64, b: u64) -> u64 {
                    a + b
                }

                pub fn subtract(env: Env, total: u128, amount: u128) -> u128 {
                    total - amount
                }

                pub fn multiply(env: Env, price: u64, qty: u64) -> u64 {
                    price * qty
                }

                pub fn safe_add(env: Env, a: u64, b: u64) -> Option<u64> {
                    a.checked_add(b)
                }
            }
        "#;
        let issues = analyzer.scan_arithmetic_overflow(source);
        // Three distinct (function, operator) pairs flagged
        assert_eq!(issues.len(), 3);

        let ops: Vec<&str> = issues.iter().map(|i| i.operation.as_str()).collect();
        assert!(ops.contains(&"+"));
        assert!(ops.contains(&"-"));
        assert!(ops.contains(&"*"));

        // safe_add uses checked_add — no bare + operator, so not flagged
        assert!(issues.iter().all(|i| i.function_name != "safe_add"));
    }

    #[test]
    fn test_scan_arithmetic_overflow_compound_assign() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn accumulate(env: Env, mut balance: u64, amount: u64) -> u64 {
                    balance += amount;
                    balance -= 1;
                    balance *= 2;
                    balance
                }
            }
        "#;
        let issues = analyzer.scan_arithmetic_overflow(source);
        // One issue per compound operator per function
        assert_eq!(issues.len(), 3);
        let ops: Vec<&str> = issues.iter().map(|i| i.operation.as_str()).collect();
        assert!(ops.contains(&"+="));
        assert!(ops.contains(&"-="));
        assert!(ops.contains(&"*="));
    }

    #[test]
    fn test_scan_arithmetic_overflow_deduplication() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn sum_three(env: Env, a: u64, b: u64, c: u64) -> u64 {
                    // Two `+` operations — should produce only ONE issue for this function
                    a + b + c
                }
            }
        "#;
        let issues = analyzer.scan_arithmetic_overflow(source);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].operation, "+");
        assert_eq!(issues[0].function_name, "sum_three");
    }

    #[test]
    fn test_scan_arithmetic_overflow_no_false_positive_safe_code() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn compare(env: Env, a: u64, b: u64) -> bool {
                    a > b
                }

                pub fn bitwise(env: Env, a: u32) -> u32 {
                    a & 0xFF
                }
            }
        "#;
        let issues = analyzer.scan_arithmetic_overflow(source);
        assert!(
            issues.is_empty(),
            "Expected no issues but found: {:?}",
            issues
        );
    }

    #[test]
    fn test_scan_arithmetic_overflow_custom_wrapper_types() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        // Custom type wrapping a primitive — arithmetic on it is still flagged
        let source = r#"
            #[contractimpl]
            impl Vault {
                pub fn add_shares(env: Env, current: Shares, delta: Shares) -> Shares {
                    Shares(current.0 + delta.0)
                }
            }
        "#;
        let issues = analyzer.scan_arithmetic_overflow(source);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].operation, "+");
    }

    /*
        #[test]
        fn test_analyze_upgrade_patterns() {
            let analyzer = Analyzer::new(SanctifyConfig::default());
            let source = r#"
                #[contracttype]
                pub enum DataKey { Admin, Balance }

                #[contractimpl]
                impl Token {
                    pub fn initialize(env: Env, admin: Address) {
                        env.storage().instance().set(&DataKey::Admin, &admin);
                    }
                    pub fn set_admin(env: Env, new_admin: Address) {
                        env.storage().instance().set(&DataKey::Admin, &new_admin);
                    }
                }
            "#;
            let report = analyzer.analyze_upgrade_patterns(source);
            assert_eq!(report.init_functions, vec!["initialize"]);
            assert_eq!(report.upgrade_mechanisms, vec!["set_admin"]);
            assert!(report.storage_types.contains(&"DataKey".to_string()));
            assert!(report
                .findings
                .iter()
                .any(|f| matches!(f.category, UpgradeCategory::Governance)));
        }
    */

    #[test]
    fn test_scan_arithmetic_overflow_suggestion_content() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn risky(env: Env, a: u64, b: u64) -> u64 {
                    a + b
                }
            }
        "#;
        let issues = analyzer.scan_arithmetic_overflow(source);
        assert_eq!(issues.len(), 1);
        // Suggestion should mention checked_add
        assert!(issues[0].suggestion.contains("checked_add"));
        // Location should include function name
        assert!(issues[0].location.starts_with("risky:"));
    }
    #[test]
    fn test_custom_rules_with_severity() {
        let config = SanctifyConfig {
            custom_rules: vec![
                CustomRule {
                    name: "no_unsafe".to_string(),
                    pattern: "unsafe".to_string(),
                    severity: crate::finding_codes::FindingSeverity::Critical,
                },
                CustomRule {
                    name: "todo_comment".to_string(),
                    pattern: "TODO".to_string(),
                    severity: RuleSeverity::Info,
                },
            ],
            ..Default::default()
        };
        let analyzer = Analyzer::new(config);
        let source = r#"
            pub fn my_fn() {
                // TODO: implement this
                unsafe {
                    let x = 1;
                }
            }
        "#;
        let matches = analyzer.analyze_custom_rules(source, &analyzer.config.custom_rules);
        assert_eq!(matches.len(), 2);

        let todo_match = matches
            .iter()
            .find(|m| m.rule_name == "todo_comment")
            .unwrap();
        assert_eq!(todo_match.severity, RuleSeverity::Info);

        let unsafe_match = matches.iter().find(|m| m.rule_name == "no_unsafe").unwrap();
        assert_eq!(
            unsafe_match.severity,
            crate::finding_codes::FindingSeverity::Critical
        );
    }

    #[test]
    fn test_unhandled_result_basic() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> u32 {
                    internal_fn()
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("not handled"));
    }

    #[test]
    fn test_unhandled_result_with_try_operator() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> Result<u32, Error> {
                    internal_fn()
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_with_unwrap() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> u32 {
                    internal_fn().unwrap()
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_with_expect() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> u32 {
                    internal_fn().expect("should succeed")
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_with_match() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> u32 {
                    match internal_fn() {
                        Ok(v) => v,
                        Err(_) => 0,
                    }
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_with_map() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) {
                    internal_fn().map(|v| v + 1);
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_private_fn() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            impl MyContract {
                fn private_fn(env: Env) -> u32 {
                    internal_fn()
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_multiple_calls() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn fn_a() -> Result<u32, Error> { Ok(1) }
            fn fn_b() -> Result<u32, Error> { Ok(2) }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) {
                    fn_a();
                    fn_b().unwrap();
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].call_expression.contains("fn_a"));
    }

    #[test]
    fn test_unhandled_result_nested_calls() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn inner() -> Result<u32, Error> { Ok(42) }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) {
                    let x = inner();
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_unhandled_result_ok_err_wrapped() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> Result<u32, Error> {
                    Ok(internal_fn()?)
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_with_unwrap_or() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            fn internal_fn() -> Result<u32, Error> {
                Ok(42)
            }

            #[contractimpl]
            impl MyContract {
                pub fn public_fn(env: Env) -> u32 {
                    internal_fn().unwrap_or(0)
                }
            }
        "#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_empty_source() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = "";
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_invalid_syntax() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = "this is not valid rust";
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(issues.len(), 0, "{:?}", issues);
    }

    #[test]
    fn test_unhandled_result_assert_macro() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
fn returns_result() -> Result<u32, Error> { Ok(42) }

#[contractimpl]
impl MyContract {
    pub fn test_fn(env: Env) {
        assert!(true);
        assert_eq!(1, 1);
        assert_ne!(1, 2);
        debug_assert!(true);
        println!("test");
        format!("test");
    }
}
"#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(
            issues.len(),
            0,
            "assert/print macros should not be flagged as unhandled results: {:?}",
            issues
        );
    }

    #[test]
    fn test_unhandled_result_fluent_builder() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
struct Builder { val: u32 }
impl Builder {
    fn with_value(mut self, v: u32) -> Self { self.val = v; self }
}

#[contractimpl]
impl MyContract {
    pub fn build(env: Env) -> u32 {
        Builder { val: 0 }
            .with_value(1)
            .with_value(2)
    }
}
"#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(
            issues.len(),
            0,
            "fluent builder should not be flagged: {:?}",
            issues
        );
    }

    #[test]
    fn test_unhandled_result_assignment() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
fn returns_result() -> Result<u32, Error> { Ok(42) }

#[contractimpl]
impl MyContract {
    pub fn test_fn(env: Env) {
        let result = returns_result();
        let _ = returns_result();
    }
}
"#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(
            issues.len(),
            2,
            "assignment without handling should be flagged: {:?}",
            issues
        );
    }

    #[test]
    fn test_unhandled_result_block_expression() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
fn returns_result() -> Result<u32, Error> { Ok(42) }

#[contractimpl]
impl MyContract {
    pub fn test_fn(env: Env) -> u32 {
        {
            let x = 1;
            x + 1
        }
    }
}
"#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(
            issues.len(),
            0,
            "block expressions should not be flagged: {:?}",
            issues
        );
    }

    #[test]
    fn test_unhandled_result_inspect() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
fn returns_result() -> Result<u32, Error> { Ok(42) }

#[contractimpl]
impl MyContract {
    pub fn test_fn(env: Env) -> u32 {
        returns_result()
            .inspect(|v| println!("Got {}", v))
            .unwrap()
    }
}
"#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(
            issues.len(),
            0,
            "inspect should be recognized as handled: {:?}",
            issues
        );
    }

    #[test]
    fn test_unhandled_result_private_fn_not_flagged() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
fn returns_result() -> Result<u32, Error> { Ok(42) }

#[contractimpl]
impl MyContract {
    fn private_fn(env: Env) {
        returns_result();
    }
}
"#;
        let issues = analyzer.scan_unhandled_results(source);
        assert_eq!(
            issues.len(),
            0,
            "private function unhandled results should not be flagged: {:?}",
            issues
        );
    }

    #[test]
    fn test_gas_estimator_simple_function() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn simple(env: Env) -> u32 {
                    42
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].function_name, "simple");
        assert_eq!(reports[0].estimated_instructions, 50);
    }

    #[test]
    fn test_gas_estimator_binary_operations() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn add(env: Env, a: u32, b: u32) -> u32 {
                    a + b
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions > 50);
    }

    #[test]
    fn test_gas_estimator_function_call() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn caller(env: Env) {
                    helper();
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions >= 70);
    }

    #[test]
    fn test_gas_estimator_storage_operations() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn store(env: Env, key: Symbol, val: u32) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions >= 1050);
    }

    #[test]
    fn test_gas_estimator_multiple_storage_ops() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn multi_store(env: Env, key: Symbol, val: u32) {
                    env.storage().persistent().set(&key, &val);
                    let exists = env.storage().persistent().has(&key);
                    env.storage().persistent().remove(&key);
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions >= 3050);
    }

    #[test]
    fn test_gas_estimator_require_auth() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn secured(env: Env, addr: Address) {
                    addr.require_auth();
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions >= 550);
    }

    #[test]
    fn test_gas_estimator_for_loop() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn iterate(env: Env, n: u32) {
                    for i in 0..n {
                        let x = i + 1;
                    }
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions > 100);
    }

    #[test]
    fn test_gas_estimator_while_loop() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn while_loop(env: Env, mut count: u32) {
                    while count > 0 {
                        count -= 1;
                    }
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions > 100);
    }

    #[test]
    fn test_gas_estimator_nested_loops() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn nested(env: Env, n: u32) {
                    for i in 0..n {
                        for j in 0..n {
                            let _ = i + j;
                        }
                    }
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions > 500);
    }

    #[test]
    fn test_gas_estimator_local_variables() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn locals(env: Env) {
                    let a: u32 = 1;
                    let b: u64 = 2;
                    let c: u128 = 3;
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_memory_bytes > 32);
    }

    #[test]
    fn test_gas_estimator_vec_macro() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn with_vec(env: Env) {
                    let v = vec![&env, 1, 2, 3];
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_memory_bytes >= 160);
    }

    #[test]
    fn test_gas_estimator_symbol_macro() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn with_symbol(env: Env) {
                    let s = symbol_short!("key");
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions >= 60);
    }

    #[test]
    fn test_gas_estimator_multiple_functions() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn func_a(env: Env) -> u32 {
                    1
                }

                pub fn func_b(env: Env) -> u32 {
                    2
                }

                fn private_func(env: Env) -> u32 {
                    3
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 2);
        let names: Vec<&str> = reports.iter().map(|r| r.function_name.as_str()).collect();
        assert!(names.contains(&"func_a"));
        assert!(names.contains(&"func_b"));
    }

    #[test]
    fn test_gas_estimator_complex_function() {
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    from.require_auth();
                    to.require_auth();
                    let balance_from: i128 = env.storage().persistent().get(&from).unwrap_or(0);
                    let balance_to: i128 = env.storage().persistent().get(&to).unwrap_or(0);
                    env.storage().persistent().set(&from, &(balance_from - amount));
                    env.storage().persistent().set(&to, &(balance_to + amount));
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions > 3000);
    }

    #[test]
    fn test_gas_estimator_empty_source() {
        let source = "";
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_gas_estimator_invalid_syntax() {
        let source = "this is not valid rust code";
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_gas_estimator_no_impl_block() {
        let source = r#"
            pub fn standalone() -> u32 {
                42
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_gas_estimator_impl_without_pub() {
        let source = r#"
            impl MyContract {
                fn private(env: Env) -> u32 {
                    42
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_gas_estimator_memory_estimation() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn memory_test(env: Env) {
                    let small: u32 = 1;
                    let medium: u64 = 2;
                    let large: u128 = 3;
                    let addr: Address = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
                    let bytes: Bytes = Bytes::new(&env);
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_memory_bytes > 100);
    }

    #[test]
    fn test_gas_estimator_conditional_logic() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn conditional(env: Env, val: u32) -> u32 {
                    if val > 10 {
                        val + 1
                    } else {
                        val - 1
                    }
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions > 50);
    }

    #[test]
    fn test_gas_estimator_match_expression() {
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn match_test(env: Env, action: u32) -> u32 {
                    match action {
                        0 => 1,
                        1 => 2,
                        _ => 0,
                    }
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].function_name, "match_test");
        assert!(reports[0].estimated_instructions >= 50);
    }

    #[test]
    fn test_gas_estimator_known_soroban_limits() {
        let source = r#"
            #[contractimpl]
            impl HeavyContract {
                pub fn heavy_storage(env: Env) {
                    env.storage().persistent().set(&Symbol::new(&env, "key1"), &1u64);
                    env.storage().persistent().set(&Symbol::new(&env, "key2"), &2u64);
                    env.storage().persistent().set(&Symbol::new(&env, "key3"), &3u64);
                    env.storage().persistent().set(&Symbol::new(&env, "key4"), &4u64);
                    env.storage().persistent().set(&Symbol::new(&env, "key5"), &5u64);
                }
            }
        "#;
        let reports = crate::gas_estimator::GasEstimator::new().estimate_contract(source);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].estimated_instructions >= 5000);
    }

    #[test]
    fn test_rule_registry_default_rules() {
        let registry = RuleRegistry::default();
        let rules = registry.available_rules();
        assert!(rules.contains(&"auth_gap"));
        assert!(rules.contains(&"ledger_size"));
        assert!(rules.contains(&"panic_detection"));
        assert!(rules.contains(&"arithmetic_overflow"));
        assert!(rules.contains(&"unhandled_result"));
    }

    #[test]
    fn test_rule_run_all() {
        let registry = RuleRegistry::default();
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn unsafe_fn(env: Env) {
                    panic!("Something went wrong");
                }
            }
        "#;
        let violations = registry.run_all(source);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_rule_run_by_name() {
        let registry = RuleRegistry::default();
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn risky(env: Env, a: u64, b: u64) -> u64 {
                    a + b
                }
            }
        "#;
        let violations = registry.run_by_name(source, "arithmetic_overflow");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_name, "arithmetic_overflow");
    }

    #[test]
    fn test_analyzer_run_rules() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn add(env: Env, a: u64, b: u64) -> u64 {
                    a + b
                }
            }
        "#;
        let violations = analyzer.run_rules(source);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_storage_collision_detects_within_same_storage_type() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn write_a(env: Env) {
                    env.storage().persistent().set(&"USER", &1u32);
                }

                pub fn write_b(env: Env) {
                    env.storage().persistent().set(&"USER", &2u32);
                }
            }
        "#;

        let collisions = analyzer.scan_storage_collisions(source);
        assert_eq!(collisions.len(), 2);
        assert!(collisions.iter().all(|c| c.key_value == "USER"));
        assert!(collisions
            .iter()
            .all(|c| c.message.contains("persistent storage key collision")));
    }

    #[test]
    fn test_storage_collision_ignores_cross_storage_type_key_reuse() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn set_persistent(env: Env) {
                    env.storage().persistent().set(&"SESSION", &1u32);
                }

                pub fn set_temporary(env: Env) {
                    env.storage().temporary().set(&"SESSION", &2u32);
                }

                pub fn set_instance(env: Env) {
                    env.storage().instance().set(&"SESSION", &3u32);
                }
            }
        "#;

        let collisions = analyzer.scan_storage_collisions(source);
        assert!(collisions.is_empty());
    }
}

impl SmtInvariantIssue {
    /// Returns the severity level of this SMT invariant violation.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::Critical
    }
}
impl SizeWarning {
    /// Returns the severity level of this ledger size warning.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        if self.level == SizeWarningLevel::ExceedsLimit {
            crate::finding_codes::FindingSeverity::Critical
        } else {
            crate::finding_codes::FindingSeverity::High
        }
    }
}
impl PanicIssue {
    /// Returns the severity level of this panic issue.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        if self.issue_type == "panic!" || self.issue_type == "unwrap" || self.issue_type == "expect"
        {
            crate::finding_codes::FindingSeverity::Critical
        } else {
            crate::finding_codes::FindingSeverity::High
        }
    }
}
impl UnsafePattern {
    /// Returns the severity level of this unsafe pattern.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::High
    }
}
impl UpgradeFinding {
    /// Returns the severity level of this upgrade finding.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::High
    }
}
impl ArithmeticIssue {
    /// Returns the severity level of this arithmetic issue.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::High
    }
}
impl StorageCollisionIssue {
    /// Returns the severity level of this storage collision issue.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::High
    }
}
impl EventIssue {
    /// Returns the severity level of this event issue.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::High
    }
}
impl UnhandledResultIssue {
    /// Returns the severity level of this unhandled result issue.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::High
    }
}

/// An authentication gap issue detected in a contract function.
#[derive(Debug, serde::Serialize, Clone, PartialEq)]
pub struct AuthGapIssue {
    /// The name of the function missing authentication.
    pub function_name: String,
}
impl AuthGapIssue {
    /// Returns the severity level of this authentication gap.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::Critical
    }
}
