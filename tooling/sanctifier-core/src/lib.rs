use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{parse_str, Fields, File, Item, Meta, Type};
use soroban_sdk::Env;
use thiserror::Error;

// ── Existing types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct SizeWarning {
    pub struct_name: String,
    pub estimated_size: usize,
    pub limit: usize,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SanctifyConfig {
    pub ignore_paths: Vec<String>,
    pub enabled_rules: Vec<String>,
    pub ledger_limit: usize,
    pub strict_mode: bool,
}

impl Default for SanctifyConfig {
    fn default() -> Self {
        Self {
            ignore_paths: vec!["target".to_string(), ".git".to_string()],
            enabled_rules: vec![
                "auth_gaps".to_string(),
                "panics".to_string(),
                "arithmetic".to_string(),
                "ledger_size".to_string(),
            ],
            ledger_limit: 64000,
            strict_mode: false,
        }
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

    pub fn scan_auth_gaps(&self, source: &str) -> Vec<String> {
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
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut issues = Vec::new();

        for item in file.items {
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

    /// Warns about `#[contracttype]` structs whose estimated size exceeds the
    /// ledger entry limit.
    pub fn analyze_ledger_size(&self, source: &str) -> Vec<SizeWarning> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut warnings = Vec::new();

        for item in file.items {
            match item {
                Item::Struct(s) => {
                    let has_contracttype = s.attrs.iter().any(|attr| {
                        if let Meta::Path(path) = &attr.meta {
                            path.is_ident("contracttype") || path.segments.iter().any(|s| s.ident == "contracttype")
                        } else {
                            false
                        }
                    });

                    if has_contracttype {
                        let size = self.estimate_struct_size(&s);
                        if size > self.config.ledger_limit
                            || (self.config.strict_mode && size > self.config.ledger_limit / 2)
                        {
                            warnings.push(SizeWarning {
                                struct_name: s.ident.to_string(),
                                estimated_size: size,
                                limit: self.config.ledger_limit,
                            });
                        }
                    }
                }
                Item::Impl(_) | Item::Macro(_) => {} // skip gracefully
                _ => {}
            }
        }

        warnings
    }

    // ── Unsafe-pattern visitor ────────────────────────────────────────────────

    /// Visitor-based scan for `panic!`, `.unwrap()`, `.expect()` with line
    /// numbers derived from proc-macro2 span locations.
    pub fn analyze_unsafe_patterns(&self, source: &str) -> Vec<UnsafePattern> {
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

    // ── Size estimation helpers ───────────────────────────────────────────────

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
                        "Vec" | "Map" => 128,
                        _ => 32,
                    }
                } else {
                    8
                }
            }
            _ => 8,
        }
    }
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
        // Should not panic during parsing
        let _ = analyzer.analyze_ledger_size(source);
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
