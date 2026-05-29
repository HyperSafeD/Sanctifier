//! S022 — Raw `invoke_contract` without `try_*` error handling.
//!
//! `env.invoke_contract()` panics when the callee returns an error, leaving no
//! opportunity for the caller to handle the failure gracefully.  Prefer
//! `env.try_invoke_contract()`, which surfaces the callee result as a typed
//! `Result`, and handle errors explicitly.

use crate::rules::{Patch, Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

/// Rule that flags raw `env.invoke_contract(…)` calls that should use
/// `env.try_invoke_contract(…)` with explicit `Result` handling.
pub struct RawInvokeContractRule;

impl RawInvokeContractRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RawInvokeContractRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RawInvokeContractRule {
    fn name(&self) -> &str {
        "raw_invoke_contract"
    }

    fn description(&self) -> &str {
        "Detects cross-contract calls via `invoke_contract` that lack `try_invoke_contract` \
         error handling — raw calls panic on callee failure instead of returning a Result"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();
        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        scan_block(&f.block, &mut violations);
                    }
                }
            }
        }
        violations
    }

    fn fix(&self, _source: &str) -> Vec<Patch> {
        vec![]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn scan_block(block: &syn::Block, violations: &mut Vec<RuleViolation>) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => scan_expr(expr, violations),
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    scan_expr(&init.expr, violations);
                }
            }
            _ => {}
        }
    }
}

fn scan_expr(expr: &syn::Expr, violations: &mut Vec<RuleViolation>) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();
            if method == "invoke_contract" {
                let line = mc.span().start().line;
                violations.push(
                    RuleViolation::new(
                        "raw_invoke_contract",
                        Severity::Warning,
                        format!(
                            "Cross-contract call via `invoke_contract` at line {} panics on \
                             callee failure; use `try_invoke_contract` with explicit Result handling",
                            line
                        ),
                        format!("line {}", line),
                    )
                    .with_suggestion(
                        "Replace `env.invoke_contract::<T>(…)` with \
                         `env.try_invoke_contract::<T, E>(…)?` or match on the returned Result \
                         to handle callee errors without panicking"
                            .to_string(),
                    ),
                );
            }
            scan_expr(&mc.receiver, violations);
            for arg in &mc.args {
                scan_expr(arg, violations);
            }
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                scan_expr(arg, violations);
            }
        }
        syn::Expr::Block(b) => scan_block(&b.block, violations),
        syn::Expr::If(i) => {
            scan_expr(&i.cond, violations);
            scan_block(&i.then_branch, violations);
            if let Some((_, else_expr)) = &i.else_branch {
                scan_expr(else_expr, violations);
            }
        }
        syn::Expr::Match(m) => {
            scan_expr(&m.expr, violations);
            for arm in &m.arms {
                scan_expr(&arm.body, violations);
            }
        }
        syn::Expr::Loop(l) => scan_block(&l.body, violations),
        syn::Expr::ForLoop(f) => scan_block(&f.body, violations),
        syn::Expr::While(w) => {
            scan_expr(&w.cond, violations);
            scan_block(&w.body, violations);
        }
        syn::Expr::Closure(c) => scan_expr(&c.body, violations),
        syn::Expr::Paren(p) => scan_expr(&p.expr, violations),
        syn::Expr::Try(t) => scan_expr(&t.expr, violations),
        syn::Expr::Await(a) => scan_expr(&a.base, violations),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_raw_invoke_contract() {
        let rule = RawInvokeContractRule::new();
        let source = r#"
            impl MyContract {
                pub fn call_other(env: Env, target: Address) {
                    let _result = env.invoke_contract::<()>(
                        &target,
                        &symbol_short!("ping"),
                        soroban_sdk::vec![&env],
                    );
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty(), "raw invoke_contract must be flagged");
        assert!(violations[0].message.contains("invoke_contract"));
        assert!(violations[0].suggestion.is_some());
    }

    #[test]
    fn does_not_flag_try_invoke_contract() {
        let rule = RawInvokeContractRule::new();
        let source = r#"
            impl MyContract {
                pub fn call_other(env: Env, target: Address) -> Result<(), Error> {
                    env.try_invoke_contract::<(), Error>(
                        &target,
                        &symbol_short!("ping"),
                        soroban_sdk::vec![&env],
                    )?;
                    Ok(())
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "try_invoke_contract must not be flagged"
        );
    }

    #[test]
    fn flags_invoke_contract_inside_if_branch() {
        let rule = RawInvokeContractRule::new();
        let source = r#"
            impl MyContract {
                pub fn conditional_call(env: Env, target: Address, flag: bool) {
                    if flag {
                        env.invoke_contract::<()>(
                            &target,
                            &symbol_short!("act"),
                            soroban_sdk::vec![&env],
                        );
                    }
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "invoke_contract inside if-branch must be flagged"
        );
    }

    #[test]
    fn suggestion_mentions_try_invoke_contract() {
        let rule = RawInvokeContractRule::new();
        let source = r#"
            impl MyContract {
                pub fn call_other(env: Env, target: Address) {
                    env.invoke_contract::<()>(
                        &target,
                        &symbol_short!("ping"),
                        soroban_sdk::vec![&env],
                    );
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty());
        let suggestion = violations[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("try_invoke_contract"),
            "suggestion must reference try_invoke_contract"
        );
    }

    #[test]
    fn empty_source_produces_no_violations() {
        let rule = RawInvokeContractRule::new();
        assert!(rule.check("").is_empty());
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = RawInvokeContractRule::new();
        assert!(rule.check("not valid rust {{{{").is_empty());
    }
}
