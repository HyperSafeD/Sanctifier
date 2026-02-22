
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::panic::{self, AssertUnwindSafe};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{parse_str, Fields, File, Item, Meta, Type};

use soroban_sdk::Env;
use thiserror::Error;

const DEFAULT_APPROACHING_THRESHOLD: f64 = 0.8;
const DEFAULT_STRICT_THRESHOLD: f64 = 0.9;

// ── Existing types ────────────────────────────────────────────────────────────

/// Severity of a ledger size warning.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum SizeWarningLevel {
    /// Size exceeds the ledger entry limit (e.g. 64KB).
    ExceedsLimit,
    /// Size is approaching the limit (configurable threshold, e.g. 80%).
    ApproachingLimit,
}

#[derive(Debug, Serialize, Clone)]
pub struct SizeWarning {
    pub struct_name: String,
    pub estimated_size: usize,
    pub limit: usize,
    pub level: SizeWarningLevel,
}

#[derive(Debug, Serialize, Clone)]
pub struct PanicIssue {
    pub function_name: String,
    pub issue_type: String, // "panic!", "unwrap", "expect"
    pub location: String,
}

// ── UnsafePattern types (visitor-based panic/unwrap scanning) ─────────────────

#[derive(Debug, Serialize, Clone)]
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

impl Default for UpgradeReport {
    fn default() -> Self {
        Self::empty()
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

// ── Configuration ─────────────────────────────────────────────────────────────

/// User-defined regex-based rule. Defined in .sanctify.toml under [[custom_rules]].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomRule {
    pub name: String,
    pub pattern: String,
}

/// A match from a custom regex rule.
#[derive(Debug, Serialize, Clone)]
pub struct CustomRuleMatch {
    pub rule_name: String,
    pub line: usize,
    pub snippet: String,
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
    #[serde(default)]
    pub strict_mode: bool,
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
    ]
}

fn default_ledger_limit() -> usize {
    64000
}

fn default_approaching_threshold() -> f64 {
    DEFAULT_APPROACHING_THRESHOLD
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

fn with_panic_guard<T>(f: impl FnOnce() -> T) -> T
where
    T: Default,
{
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => T::default(),
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
    } else if size > approaching {
        Some(SizeWarningLevel::ApproachingLimit)
    } else if strict && size > strict_threshold {
        Some(SizeWarningLevel::ApproachingLimit)
    } else {
        None
    }
}

// ── Analyzer ──────────────────────────────────────────────────────────────────

pub struct Analyzer {
    pub config: SanctifyConfig,
}

impl Analyzer {
    pub fn new(config: SanctifyConfig) -> Self {
        Self { config }
    }

    pub fn analyze_upgrade_patterns(&self, source: &str) -> UpgradeReport {
        with_panic_guard(|| self.analyze_upgrade_patterns_impl(source))
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
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        self.check_fn_panics(&f.block, &fn_name, &mut issues);
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
                    if m.mac.path.is_ident("require_auth") || m.mac.path.is_ident("require_auth_for_args") {
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
                    // Heuristic: check if receiver chain contains "storage"
                    let receiver_str = quote::quote!(#m.receiver).to_string();
                    if receiver_str.contains("storage") || receiver_str.contains("persistent") || receiver_str.contains("temporary") || receiver_str.contains("instance") {
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

    // ── Storage collision (stub) ──────────────────────────────────────────────

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
        let approaching = ((limit as f64) * self.config.approaching_threshold) as usize;
        let strict = self.config.strict_mode;
        let strict_threshold = ((limit as f64) * DEFAULT_STRICT_THRESHOLD) as usize;

        let mut warnings = Vec::new();
        for item in &file.items {
            match item {
                Item::Struct(s) => {
                    if has_contracttype(&s.attrs) {
                        let size = self.estimate_struct_size(s);
                        if let Some(level) = classify_size(size, limit, approaching, strict, strict_threshold) {
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
                        if let Some(level) = classify_size(size, limit, approaching, strict, strict_threshold) {
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
                    });
                }
            }
        }
        matches
    }

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
                                let inner: usize = args.args.iter().filter_map(|a| {
                                    if let syn::GenericArgument::Type(t) = a {
                                        Some(self.estimate_type_size(t))
                                    } else {
                                        None
                                    }
                                }).sum();
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

#[derive(Debug, Clone, Serialize)]
pub struct ContractCallEdge {
    pub caller: String,
    pub callee: String,
    pub file: String,
    pub line: usize,
    pub contract_id_expr: String,
    pub function_expr: Option<String>,
}

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

// ── UnsafeVisitor ─────────────────────────────────────────────────────────────

struct UnsafeVisitor {
    patterns: Vec<UnsafePattern>,
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_expr_method_call(&mut self, i: &'ast syn::ExprMethodCall) {
        let method_name = i.method.to_string();
        if method_name == "unwrap" || method_name == "expect" {
            let pattern_type = if method_name == "unwrap" {
                PatternType::Unwrap
            } else {
                PatternType::Expect
            };
            self.patterns.push(UnsafePattern {
                pattern_type,
                snippet: quote::quote!(#i).to_string(),
                line: 0, // Simplified for now
            });
        }
        visit::visit_expr_method_call(self, i);
    }

    fn visit_expr_macro(&mut self, i: &'ast syn::ExprMacro) {
        if i.mac.path.is_ident("panic") {
            self.patterns.push(UnsafePattern {
                pattern_type: PatternType::Panic,
                snippet: quote::quote!(#i).to_string(),
                line: 0,
            });
        }
        visit::visit_expr_macro(self, i);
    }
}

// ── SanctifiedGuard (runtime monitoring) ───────────────────────────────────────

/// Error type for SanctifiedGuard runtime invariant violations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invariant violation: {0}")]
    InvariantViolation(String),
}

/// Trait for runtime monitoring. Implement this to enforce invariants
/// on your contract state. The foundation for runtime monitoring.
pub trait SanctifiedGuard {
    /// Verifies that contract invariants hold in the current environment.
    /// Returns `Ok(())` if all invariants hold, or `Err` with a violation message.
    fn check_invariant(&self, env: &Env) -> Result<(), Error>;
}

// ── ArithVisitor ──────────────────────────────────────────────────────────────

struct ArithVisitor {
    issues: Vec<ArithmeticIssue>,
    /// Name of the function currently being visited.
    current_fn: Option<String>,
    /// De-duplicates issues: one per (function_name, operator) pair.
    seen: HashSet<(String, String)>,
}

impl ArithVisitor {
    /// Returns `(operator_str, suggestion_text)` for overflow-prone binary ops,
    /// or `None` for operators that cannot overflow (comparisons, bitwise, etc).
    fn classify_op(op: &syn::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            syn::BinOp::Add(_) => Some((
                "+",
                "Use `.checked_add(rhs)` or `.saturating_add(rhs)` to handle overflow",
            )),
            syn::BinOp::Sub(_) => Some((
                "-",
                "Use `.checked_sub(rhs)` or `.saturating_sub(rhs)` to handle underflow",
            )),
            syn::BinOp::Mul(_) => Some((
                "*",
                "Use `.checked_mul(rhs)` or `.saturating_mul(rhs)` to handle overflow",
            )),
            syn::BinOp::AddAssign(_) => Some((
                "+=",
                "Replace `a += b` with `a = a.checked_add(b).expect(\"overflow\")`",
            )),
            syn::BinOp::SubAssign(_) => Some((
                "-=",
                "Replace `a -= b` with `a = a.checked_sub(b).expect(\"underflow\")`",
            )),
            syn::BinOp::MulAssign(_) => Some((
                "*=",
                "Replace `a *= b` with `a = a.checked_mul(b).expect(\"overflow\")`",
            )),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for ArithVisitor {
    /// Track the current function when descending into an impl method.
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    /// Also handle top-level `fn` items (helper functions outside impls).
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if let Some(fn_name) = self.current_fn.clone() {
            if let Some((op_str, suggestion)) = Self::classify_op(&node.op) {
                // Skip concatenation of string literals (false positive for `+`)
                if !is_string_literal(&node.left) && !is_string_literal(&node.right) {
                    let key = (fn_name.clone(), op_str.to_string());
                    if !self.seen.contains(&key) {
                        self.seen.insert(key);
                        // Line number from the left operand's span
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
        // Continue descending so nested binary ops are also checked
        visit::visit_expr_binary(self, node);
    }
}

/// Returns `true` if the expression is a string literal — used to avoid
/// false-positives on `+` for string concatenation (rare in no_std Soroban
/// but included for correctness).
fn is_string_literal(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        })
    )
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
        let mut config = SanctifyConfig::default();
        config.ledger_limit = 50;
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
        assert_eq!(gaps[0], "set_data");
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
}
pub mod gas_estimator;
pub mod gas_report;
