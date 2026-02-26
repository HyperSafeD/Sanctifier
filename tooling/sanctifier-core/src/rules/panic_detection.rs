use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File};

pub struct PanicDetectionRule;

impl PanicDetectionRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PanicDetectionRule {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PanicIssue {
    pub function_name: String,
    pub issue_type: String,
    pub location: String,
}

impl Rule for PanicDetectionRule {
    fn name(&self) -> &str {
        "panic_detection"
    }

    fn description(&self) -> &str {
        "Detects panic!, unwrap(), and expect() calls that can cause contract failures"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut issues = Vec::new();
        for item in &file.items {
            if let syn::Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();
                        check_fn_panics(&f.block, &fn_name, &mut issues);
                    }
                }
            }
        }

        issues
            .into_iter()
            .map(|issue| {
                let severity = match issue.issue_type.as_str() {
                    "panic!" => Severity::Error,
                    _ => Severity::Warning,
                };
                RuleViolation::new(
                    self.name(),
                    severity,
                    format!("Use of '{}' can cause contract failure", issue.issue_type),
                    issue.location,
                )
                .with_suggestion("Use Result types and proper error handling instead".to_string())
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn check_fn_panics(block: &syn::Block, fn_name: &str, issues: &mut Vec<PanicIssue>) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => check_expr_panics(expr, fn_name, issues),
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr_panics(&init.expr, fn_name, issues);
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

fn check_expr_panics(expr: &syn::Expr, fn_name: &str, issues: &mut Vec<PanicIssue>) {
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
            check_expr_panics(&m.receiver, fn_name, issues);
            for arg in &m.args {
                check_expr_panics(arg, fn_name, issues);
            }
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                check_expr_panics(arg, fn_name, issues);
            }
        }
        syn::Expr::Block(b) => check_fn_panics(&b.block, fn_name, issues),
        syn::Expr::If(i) => {
            check_expr_panics(&i.cond, fn_name, issues);
            check_fn_panics(&i.then_branch, fn_name, issues);
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr_panics(else_expr, fn_name, issues);
            }
        }
        syn::Expr::Match(m) => {
            check_expr_panics(&m.expr, fn_name, issues);
            for arm in &m.arms {
                check_expr_panics(&arm.body, fn_name, issues);
            }
        }
        _ => {}
    }
}

use serde::Serialize;
