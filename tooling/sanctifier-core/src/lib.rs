use regex::Regex;
use serde::{Deserialize, Serialize};
use soroban_sdk::Env;
use std::collections::HashSet;
use std::panic::{self, AssertUnwindSafe};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, Fields, File, Item, Meta, Type};
use thiserror::Error;

pub mod complexity;
pub mod finding_codes;
pub mod gas_estimator;
pub mod gas_report;
pub mod patcher;
pub mod reentrancy;
pub mod rules;
pub mod sdk_version;
pub mod sep41;
#[cfg(feature = "smt")]
pub mod smt;
pub mod soroban_v21;
pub mod storage_collision;

// Re-export common types for easier CLI access
pub use complexity::{ContractMetrics, FunctionMetrics};
pub use finding_codes::FindingSeverity as RuleSeverity;
pub use finding_codes::FindingSeverity;
pub use reentrancy::ReentrancyEdge;
pub use rules::{Patch, Rule, RuleRegistry, RuleViolation, Severity};
pub use sep41::{Sep41Issue, Sep41IssueKind, Sep41VerificationReport};
#[cfg(feature = "smt")]
pub use smt::SmtInvariantIssue;
pub use storage_collision::StorageCollisionIssue;

// ── Panic Guard ───────────────────────────────────────────────────────────────

/// Runs analysis logic inside a panic guard. Returns empty/default on panic,
/// e.g. when complex macros (contractimpl, etc.) cause AST parsing to fail.
fn with_panic_guard<T, F>(f: F) -> T
where
    F: FnOnce() -> T + panic::UnwindSafe,
    T: Default,
{
    panic::catch_unwind(AssertUnwindSafe(f)).unwrap_or_default()
}

// ── Configuration ─────────────────────────────────────────────────────────────

pub const DEFAULT_LEDGER_ENTRY_LIMIT: usize = 64 * 1024;
pub const DEFAULT_APPROACHING_THRESHOLD: f64 = 0.8;

/// User-defined regex-based rule. Defined in .sanctify.toml under [[rules]].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomRule {
    pub name: String,
    pub pattern: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub severity: finding_codes::FindingSeverity,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SanctifyConfig {
    #[serde(default = "default_ignore_paths")]
    pub ignore_paths: Vec<String>,
    #[serde(default = "default_enabled_rules")]
    pub enabled_rules: Vec<String>,
    #[serde(default = "default_ledger_limit")]
    pub ledger_limit: usize,
    #[serde(default = "default_approaching_threshold")]
    pub approaching_threshold: f64,
    #[serde(default = "default_telemetry_enabled")]
    pub telemetry: bool,
    #[serde(default)]
    pub strict_mode: bool,
    /// Custom regex rules (field name "rules" in TOML).
    #[serde(default, alias = "custom_rules")]
    pub rules: Vec<CustomRule>,
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
    ]
}
fn default_ledger_limit() -> usize {
    DEFAULT_LEDGER_ENTRY_LIMIT
}
fn default_approaching_threshold() -> f64 {
    DEFAULT_APPROACHING_THRESHOLD
}

fn default_telemetry_enabled() -> bool {
    false
}

impl Default for SanctifyConfig {
    fn default() -> Self {
        Self {
            ignore_paths: default_ignore_paths(),
            enabled_rules: default_enabled_rules(),
            ledger_limit: default_ledger_limit(),
            approaching_threshold: default_approaching_threshold(),
            telemetry: default_telemetry_enabled(),
            strict_mode: false,
            rules: vec![],
        }
    }
}

// ── Finding types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum SizeWarningLevel {
    ExceedsLimit,
    ApproachingLimit,
}

#[derive(Debug, Serialize, Clone)]
pub struct SizeWarning {
    pub struct_name: String,
    pub estimated_size: usize,
    pub limit: usize,
    pub level: SizeWarningLevel,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
pub enum PatternType {
    Panic,
    Unwrap,
    Expect,
}

#[derive(Debug, Serialize, Clone)]
pub struct UnsafePattern {
    pub pattern_type: PatternType,
    pub line: usize,
    pub snippet: String,
}

// ── Upgrade analysis types ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct UpgradeFinding {
    pub category: UpgradeCategory,
    pub function_name: Option<String>,
    pub location: String,
    pub message: String,
    pub suggestion: String,
    pub severity: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeCategory {
    AdminControl,
    Timelock,
    InitPattern,
    StorageLayout,
    Governance,
}

/// Upgrade safety report.
#[derive(Debug, Serialize, Clone)]
pub struct UpgradeReport {
    pub findings: Vec<UpgradeFinding>,
    pub upgrade_mechanisms: Vec<String>,
    pub init_functions: Vec<String>,
    pub storage_types: Vec<String>,
    pub suggestions: Vec<String>,
}

impl UpgradeReport {
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

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        if let Meta::Path(path) = &attr.meta {
            path.is_ident(name) || path.segments.iter().any(|s| s.ident == name)
        } else {
            false
        }
    })
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
pub struct PanicIssue {
    pub function_name: String,
    pub issue_type: String,
    pub location: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ArithmeticIssue {
    pub function_name: String,
    pub operation: String,
    pub suggestion: String,
    pub location: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TruncationBoundsIssue {
    pub function_name: String,
    pub kind: String,
    pub expression: String,
    pub suggestion: String,
    pub location: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AuthGapIssue {
    pub function_name: String,
    pub location: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventIssue {
    pub event_name: String,
    pub issue_type: String,
    pub location: String,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct UnhandledResultIssue {
    pub function_name: String,
    pub call_expression: String,
    pub location: String,
    pub message: String,
}

/// A match from a custom regex rule.
#[derive(Debug, Serialize, Clone)]
pub struct CustomRuleMatch {
    pub rule_name: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct GasEstimation {
    pub function_name: String,
    pub estimated_gas: u64,
    pub complexity_score: usize,
}

// ── Runtime Monitoring ────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum Error {
    #[error("invariant violation: {0}")]
    InvariantViolation(String),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Trait for runtime monitoring. Implement this to enforce invariants on your contract state.
pub trait SanctifiedGuard {
    fn check_invariant(&self, env: &Env) -> Result<(), Error>;
}

#[derive(Debug, Serialize, Clone)]
pub struct Finding {
    pub severity: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}

// ── Analyzer ──────────────────────────────────────────────────────────────────

pub struct Analyzer {
    pub config: SanctifyConfig,
}

impl Analyzer {
    pub fn new(config: SanctifyConfig) -> Self {
        Self { config }
    }
    pub fn run_rule(&self, source: &str, rule_name: &str) -> Vec<RuleViolation> {
        let registry = rules::RuleRegistry::with_default_rules();
        registry.run_by_name(source, rule_name)
    }

    pub fn verify_sep41_interface(&self, source: &str) -> sep41::Sep41VerificationReport {
        sep41::verify(source)
    }

    pub fn analyze_complexity(
        &self,
        source: &str,
        path: &str,
    ) -> Result<complexity::ContractMetrics, syn::Error> {
        complexity::analyze_complexity_from_source(source, path)
    }

    pub fn scan_invoke_contract_calls(
        &self,
        source: &str,
        _contract_name: &str,
        _file_path: &str,
    ) -> Vec<reentrancy::ReentrancyEdge> {
        reentrancy::scan_invoke_contract_calls(source)
    }

    pub fn analyze_custom_rules(&self, source: &str) -> Vec<CustomRuleMatch> {
        let mut matches = Vec::new();
        for rule in &self.config.rules {
            let re = match Regex::new(&rule.pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for (line_no, line) in source.lines().enumerate() {
                if re.find(line).is_some() {
                    matches.push(CustomRuleMatch {
                        rule_name: rule.name.clone(),
                        line: line_no + 1,
                        snippet: line.trim().to_string(),
                    });
                }
            }
        }
        matches
    }

    pub fn scan_auth_gaps(&self, source: &str) -> Vec<String> {
        with_panic_guard(|| self.scan_auth_gaps_impl(source))
    }

    fn scan_auth_gaps_impl(&self, source: &str) -> Vec<String> {
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
                            let mut has_mutation = false;
                            let mut has_auth = false;
                            self.check_fn_body(&f.block, &mut has_mutation, &mut has_auth);
                            if has_mutation && !has_auth {
                                gaps.push(fn_name);
                            }
                        }
                    }
                }
            }
        }
        gaps
    }

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
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        self.check_fn_panics(&f.block, &f.sig.ident.to_string(), &mut issues);
                    }
                }
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

    fn check_fn_body(&self, block: &syn::Block, has_mutation: &mut bool, has_auth: &mut bool) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => self.check_expr(expr, has_mutation, has_auth),
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.check_expr(&init.expr, has_mutation, has_auth);
                    }
                }
                syn::Stmt::Macro(m) => {
                    if m.mac.path.is_ident("require_auth")
                        || m.mac.path.is_ident("require_auth_for_args")
                    {
                        *has_auth = true;
                    }
                }
                _ => {}
            }
        }
    }

    fn check_expr(&self, expr: &syn::Expr, has_mutation: &mut bool, has_auth: &mut bool) {
        match expr {
            syn::Expr::Call(c) => {
                if let syn::Expr::Path(p) = &*c.func {
                    if let Some(segment) = p.path.segments.last() {
                        let ident = segment.ident.to_string();
                        if ident == "require_auth" || ident == "require_auth_for_args" {
                            *has_auth = true;
                        }
                    }
                }
                for arg in &c.args {
                    self.check_expr(arg, has_mutation, has_auth);
                }
            }
            syn::Expr::MethodCall(m) => {
                let method_name = m.method.to_string();
                if method_name == "set" || method_name == "update" || method_name == "remove" {
                    let receiver_str = quote::quote!(#m.receiver).to_string();
                    if receiver_str.contains("storage")
                        || receiver_str.contains("persistent")
                        || receiver_str.contains("temporary")
                        || receiver_str.contains("instance")
                    {
                        *has_mutation = true;
                    }
                }
                if method_name == "require_auth" || method_name == "require_auth_for_args" {
                    *has_auth = true;
                }
                self.check_expr(&m.receiver, has_mutation, has_auth);
                for arg in &m.args {
                    self.check_expr(arg, has_mutation, has_auth);
                }
            }
            syn::Expr::Block(b) => self.check_fn_body(&b.block, has_mutation, has_auth),
            syn::Expr::If(i) => {
                self.check_expr(&i.cond, has_mutation, has_auth);
                self.check_fn_body(&i.then_branch, has_mutation, has_auth);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr(else_expr, has_mutation, has_auth);
                }
            }
            syn::Expr::Match(m) => {
                self.check_expr(&m.expr, has_mutation, has_auth);
                for arm in &m.arms {
                    self.check_expr(&arm.body, has_mutation, has_auth);
                }
            }
            _ => {}
        }
    }

    pub fn scan_storage_collisions(&self, source: &str) -> Vec<StorageCollisionIssue> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let mut visitor = storage_collision::StorageVisitor::new();
        syn::visit::visit_file(&mut visitor, &file);
        visitor.final_check();
        visitor.collisions
    }

    pub fn scan_events(&self, _source: &str) -> Vec<EventIssue> {
        // Event scanning is not implemented in the current core engine.
        Vec::new()
    }

    pub fn scan_unhandled_results(&self, source: &str) -> Vec<UnhandledResultIssue> {
        with_panic_guard(|| {
            self.run_rule(source, "unhandled_result")
                .into_iter()
                .map(|violation| UnhandledResultIssue {
                    function_name: String::new(),
                    call_expression: String::new(),
                    location: violation.location,
                    message: violation.message,
                })
                .collect()
        })
    }

    pub fn analyze_ledger_size(&self, source: &str) -> Vec<SizeWarning> {
        with_panic_guard(|| self.analyze_ledger_size_impl(source))
    }

    fn analyze_ledger_size_impl(&self, source: &str) -> Vec<SizeWarning> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let mut warnings = Vec::new();
        let limit = self.config.ledger_limit;
        let approaching = (limit as f64 * self.config.approaching_threshold) as usize;
        let strict = self.config.strict_mode;
        let strict_threshold = limit / 2;

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
                _ => {}
            }
        }
        warnings
    }

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
        };
        visitor.visit_file(&file);
        visitor.issues
    }

    pub fn scan_gas_estimation(&self, source: &str) -> Vec<GasEstimation> {
        let reports = gas_estimator::GasEstimator::new().estimate_contract(source);
        reports
            .into_iter()
            .map(|r| GasEstimation {
                function_name: r.function_name,
                estimated_gas: r.estimated_instructions as u64,
                complexity_score: 0,
            })
            .collect()
    }

    // ── Upgrade pattern analysis ──────────────────────────────────────────────

    /// Analyzes contracts for upgrade mechanisms, init patterns, storage layout
    /// visibility, and governance (auth on privileged functions).
    pub fn analyze_upgrade_patterns(&self, source: &str) -> UpgradeReport {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return UpgradeReport::empty(),
        };

        let mut report = UpgradeReport {
            findings: Vec::new(),
            upgrade_mechanisms: Vec::new(),
            init_functions: Vec::new(),
            storage_types: Vec::new(),
            suggestions: Vec::new(),
        };

        // Collect #[contracttype] storage types
        for item in &file.items {
            if let Item::Struct(s) = item {
                if has_attr(&s.attrs, "contracttype") {
                    report.storage_types.push(s.ident.to_string());
                }
            }
            if let Item::Enum(e) = item {
                if has_attr(&e.attrs, "contracttype") {
                    report.storage_types.push(e.ident.to_string());
                }
            }
        }

        // Walk impl blocks for upgrade-related functions
        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        let line = f.sig.ident.span().start().line;
                        let location = format!("{}:{}", fn_name, line);

                        // Admin / upgrade mechanism detection
                        if is_upgrade_or_admin_fn(&fn_name) {
                            report.upgrade_mechanisms.push(fn_name.clone());
                            let has_auth = self.fn_has_auth(&f.block);
                            if !has_auth {
                                report.findings.push(UpgradeFinding {
                                    category: UpgradeCategory::Governance,
                                    function_name: Some(fn_name.clone()),
                                    location: location.clone(),
                                    message: format!(
                                        "Upgrade/admin function '{}' modifies state without require_auth",
                                        fn_name
                                    ),
                                    suggestion: "Add require_auth or require_auth_for_args before state mutations".to_string(),
                                    severity: "high".to_string(),
                                });
                                report.suggestions.push(format!(
                                    "Ensure '{}' requires admin authorization",
                                    fn_name
                                ));
                            }
                        }

                        // Init pattern detection
                        if is_init_fn(&fn_name) {
                            report.init_functions.push(fn_name.clone());
                            let has_guard = fn_has_reinit_guard(&f.block);
                            let severity_str = if has_guard { "medium".to_string() } else { "critical".to_string() };
                            let message = if has_guard {
                                format!("Initialization function '{}' detected (with re-init guard)", fn_name)
                            } else {
                                format!("Initialization function '{}' is callable more than once — add re-init guard", fn_name)
                            };
                            report.findings.push(UpgradeFinding {
                                category: UpgradeCategory::InitPattern,
                                function_name: Some(fn_name.clone()),
                                location: location.clone(),
                                message,
                                suggestion: "Guard init with an early return when storage already has an initialization flag: if env.storage().instance().has(&DataKey::IsInit) { return; }".to_string(),
                                severity: severity_str,
                            });
                        }

                        // Timelock heuristics: look for delay/timelock in name or body
                        if fn_name.to_lowercase().contains("upgrade")
                            && self.fn_references_delay(&f.block)
                        {
                            report.findings.push(UpgradeFinding {
                                category: UpgradeCategory::Timelock,
                                function_name: Some(fn_name.clone()),
                                location: location.clone(),
                                message: format!(
                                    "Upgrade function '{}' may use delay/timelock",
                                    fn_name
                                ),
                                suggestion: "Verify timelock delay is enforced before upgrade"
                                    .to_string(),
                                    severity: "medium".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Storage layout suggestions
        if report.storage_types.len() > 1 {
            report.suggestions.push(format!(
                "Track storage types [{}] across versions; avoid reordering or removing fields",
                report.storage_types.join(", ")
            ));
        }

        report
    }

    fn fn_has_auth(&self, block: &syn::Block) -> bool {
        let mut has = false;
        self.check_fn_body(block, &mut false, &mut has);
        has
    }

    fn fn_references_delay(&self, block: &syn::Block) -> bool {
        let s = quote::quote!(#block).to_string();
        s.contains("delay") || s.contains("timelock") || s.contains("ledger_seq")
    }

    // ── Size estimation helpers ───────────────────────────────────────────────

    fn estimate_enum_size(&self, e: &syn::ItemEnum) -> usize {
        const DISCRIMINANT_SIZE: usize = 4;
        let mut max_variant = 0usize;
        for v in &e.variants {
            let mut variant_size = 0;
            match &v.fields {
                Fields::Named(fields) => {
                    for f in &fields.named {
                        variant_size += self.estimate_type_size(&f.ty);
                    }
                }
                Fields::Unnamed(fields) => {
                    for f in &fields.unnamed {
                        variant_size += self.estimate_type_size(&f.ty);
                    }
                }
                Fields::Unit => {}
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
                    match seg.ident.to_string().as_str() {
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
                    }
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

fn has_contracttype(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if let Meta::Path(path) = &attr.meta {
            path.is_ident("contracttype") || path.segments.iter().any(|s| s.ident == "contracttype")
        } else {
            false
        }
    })
}

fn classify_size(
    size: usize,
    limit: usize,
    approaching: usize,
    strict: bool,
    strict_threshold: usize,
) -> Option<SizeWarningLevel> {
    if size > limit {
        Some(SizeWarningLevel::ExceedsLimit)
    } else if size > approaching || (strict && size > strict_threshold) {
        Some(SizeWarningLevel::ApproachingLimit)
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
                pub buffer: Bytes,
            }
        "#;
        let warnings = analyzer.analyze_ledger_size(source);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].struct_name, "ExceedsLimit");
        assert_eq!(warnings[0].level, SizeWarningLevel::ExceedsLimit);
    }

    #[test]
    fn test_complex_macro_no_panic() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #[contractimpl]
            impl Contract {
                pub fn test(_env: Env) {
                    let _x = 1u32;
                }
            }
        "#;
        // Must not panic
        let _ = analyzer.analyze_ledger_size(source);
        let _ = analyzer.scan_auth_gaps(source);
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
            }
        "#;
        let gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0], "set_data");
    }
}

// ── Visitors ──────────────────────────────────────────────────────────────────

struct UnsafeVisitor {
    patterns: Vec<UnsafePattern>,
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if node.path.is_ident("panic") {
            let line = node
                .path
                .get_ident()
                .map(|i| i.span().start().line)
                .unwrap_or(0);
            self.patterns.push(UnsafePattern {
                pattern_type: PatternType::Panic,
                line,
                snippet: "panic!()".to_string(),
            });
        }
        syn::visit::visit_macro(self, node);
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method = node.method.to_string();
        if method == "unwrap" || method == "expect" {
            let line = node.method.span().start().line;
            let pattern_type = if method == "unwrap" {
                PatternType::Unwrap
            } else {
                PatternType::Expect
            };
            self.patterns.push(UnsafePattern {
                pattern_type,
                line,
                snippet: format!(".{}()", method),
            });
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

// ── Tests (Continued) ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_continued {
    use super::*;

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
    fn test_token_with_bugs() {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let source = r#"
            #![no_std]
            use soroban_sdk::{contract, contractimpl, symbol_short, Env, String, Address, Symbol, Val};

            #[contract]
            pub struct TokenWithBugs;

            const BALANCE: Symbol = symbol_short!("BALANCE");

            #[contractimpl]
            impl TokenWithBugs {
                pub fn initialize(e: Env, admin: Address, name: String, symbol: String) {
                    // Not implemented for this test
                }

                pub fn balance(e: Env, id: Address) -> i128 {
                    e.storage().persistent().get(&id).unwrap_or(0)
                }

                // This transfer function is missing an authorization check but performs a storage operation
                pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
                    // Vulnerability: Missing require_auth call for 'from'
                    let from_balance = Self::balance(e.clone(), from.clone());
                    e.storage().persistent().set(&from, &(from_balance - amount)); // Mutable operation
                    
                    let to_balance = Self::balance(e.clone(), to.clone());
                    e.storage().persistent().set(&to, &(to_balance + amount));
                }

                // This mint function can cause an overflow
                pub fn mint(e: Env, to: Address, amount: i128) {
                    // VULNERABILITY: No overflow check
                    let current_balance = Self::balance(e.clone(), to.clone());
                    let new_balance = current_balance + amount; // This can overflow
                    e.storage().persistent().set(&to, &new_balance);
                }

                pub fn symbol(e: Env) -> String {
                    String::from_str(&e, "TKN")
                }
            }
        "#;

        let auth_gaps = analyzer.scan_auth_gaps(source);
        assert_eq!(auth_gaps.len(), 2, "Expected 2 auth gaps");
        assert!(auth_gaps.contains(&"transfer".to_string()));
        assert!(auth_gaps.contains(&"mint".to_string()));

        let arithmetic_issues = analyzer.scan_arithmetic_overflow(source);
        assert_eq!(arithmetic_issues.len(), 3, "Expected 3 arithmetic issues");
        assert!(arithmetic_issues
            .iter()
            .any(|issue| issue.function_name == "transfer" && issue.operation == "-"));
        assert!(arithmetic_issues
            .iter()
            .any(|issue| issue.function_name == "transfer" && issue.operation == "+"));
        assert!(arithmetic_issues
            .iter()
            .any(|issue| issue.function_name == "mint" && issue.operation == "+"));
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
}

struct ArithVisitor {
    issues: Vec<ArithmeticIssue>,
    current_fn: Option<String>,
    seen: HashSet<(String, String)>,
}

impl ArithVisitor {
    fn classify_op(op: &syn::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            syn::BinOp::Add(_) => Some(("+", "Use `.checked_add(rhs)` or `.saturating_add(rhs)`")),
            syn::BinOp::Sub(_) => Some(("-", "Use `.checked_sub(rhs)` or `.saturating_sub(rhs)`")),
            syn::BinOp::Mul(_) => Some(("*", "Use `.checked_mul(rhs)` or `.saturating_mul(rhs)`")),
            syn::BinOp::AddAssign(_) => Some((
                "+=",
                "Replace with `a = a.checked_add(b).expect(\"overflow\")`",
            )),
            syn::BinOp::SubAssign(_) => Some((
                "-=",
                "Replace with `a = a.checked_sub(b).expect(\"underflow\")`",
            )),
            syn::BinOp::MulAssign(_) => Some((
                "*=",
                "Replace with `a = a.checked_mul(b).expect(\"overflow\")`",
            )),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for ArithVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if let Some(fn_name) = self.current_fn.clone() {
            if let Some((op_str, suggestion)) = Self::classify_op(&node.op) {
                if !is_string_literal(&node.left) && !is_string_literal(&node.right) {
                    let key = (fn_name.clone(), op_str.to_string());
                    if !self.seen.contains(&key) {
                        self.seen.insert(key);
                        let line = node.left.span().start().line;
                        self.issues.push(ArithmeticIssue {
                            function_name: fn_name.clone(),
                            operation: op_str.to_string(),
                            suggestion: suggestion.to_string(),
                            location: format!("{}:{}", fn_name, line),
                        });
                    }
                }
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

fn is_string_literal(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        })
    )
}

/// Check if a function body contains an early-return guard against re-initialization.
/// Looks for a pattern where storage is queried (via `.has()`, `.get()`, or `.try_get()`)
/// and an early exit (`return` or `panic!`) follows before the main init logic.
fn fn_has_reinit_guard(block: &syn::Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                if expr_has_storage_guard(expr) {
                    return true;
                }
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    if expr_has_storage_guard(&init.expr) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Recursively check if an expression contains a storage guard pattern.
/// A storage guard is an `if` whose condition calls `.has()`, `.get()`, or `.try_get()`
/// on a storage API, and whose branch contains a `return` or `panic!`.
fn expr_has_storage_guard(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::If(i) => {
            let cond_str = quote::quote!(#i.cond).to_string();
            let has_storage_check = cond_str.contains(".has(")
                || cond_str.contains(".get(")
                || cond_str.contains(".try_get(");
            if !has_storage_check {
                // Check sub-expressions recursively
                return expr_has_storage_guard(&i.cond)
                    || block_has_early_exit(&i.then_branch);
            }
            // Check if either branch has an early return/panic
            if block_has_early_exit(&i.then_branch) {
                return true;
            }
            if let Some((_, else_expr)) = &i.else_branch {
                if expr_has_early_exit(else_expr) {
                    return true;
                }
            }
            true // storage check found — treat as guarded regardless of branch content
        }
        syn::Expr::Block(b) => fn_has_reinit_guard(&b.block),
        syn::Expr::Unary(u) => expr_has_storage_guard(&u.expr),
        syn::Expr::Paren(p) => expr_has_storage_guard(&p.expr),
        syn::Expr::Binary(b) => {
            expr_has_storage_guard(&b.left) || expr_has_storage_guard(&b.right)
        }
        _ => false,
    }
}

fn block_has_early_exit(block: &syn::Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                if expr_has_early_exit(expr) {
                    return true;
                }
            }
            syn::Stmt::Macro(m) => {
                if m.mac.path.is_ident("panic") {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn expr_has_early_exit(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Return(_) => true,
        syn::Expr::Macro(m) => m.mac.path.is_ident("panic"),
        syn::Expr::Block(b) => block_has_early_exit(&b.block),
        syn::Expr::If(i) => {
            block_has_early_exit(&i.then_branch)
                || i.else_branch
                    .as_ref()
                    .map(|(_, e)| expr_has_early_exit(e))
                    .unwrap_or(false)
        }
        _ => false,
    }
}
