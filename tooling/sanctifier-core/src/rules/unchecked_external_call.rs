use crate::rules::{Patch, Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

/// Rule that detects unchecked return values from external Soroban calls.
pub struct UncheckedExternalCallRule;

impl UncheckedExternalCallRule {
    /// Create a new unchecked-external-call rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for UncheckedExternalCallRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for UncheckedExternalCallRule {
    fn name(&self) -> &str {
        "unchecked_external_call"
    }

    fn description(&self) -> &str {
        "Detects external cross-contract calls whose Result return values are not checked or handled"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();
        for item in &file.items {
            if let Item::Impl(i) = item {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        check_function_for_unchecked_calls(&f.block, &mut violations);
                    }
                }
            }
        }
        violations
    }

    fn fix(&self, _source: &str) -> Vec<Patch> {
        // Auto-fix is complex for this rule - requires context-specific error handling
        vec![]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn check_function_for_unchecked_calls(block: &syn::Block, violations: &mut Vec<RuleViolation>) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, Some(_)) => {
                // Expression statement with semicolon - check if it's an external call
                if is_unchecked_external_call(expr) {
                    let span = expr.span();
                    violations.push(
                        RuleViolation::new(
                            "unchecked_external_call",
                            Severity::Warning,
                            "External contract call result is not checked or handled".to_string(),
                            format!("line {}", span.start().line),
                        )
                        .with_suggestion(
                            "Store the result in a variable and handle errors with match, ?, or .unwrap_or()".to_string()
                        ),
                    );
                }
                check_expr_recursively(expr, violations);
            }
            syn::Stmt::Expr(expr, None) => {
                check_expr_recursively(expr, violations);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr_recursively(&init.expr, violations);
                }
            }
            _ => {}
        }
    }
}

fn check_expr_recursively(expr: &syn::Expr, violations: &mut Vec<RuleViolation>) {
    match expr {
        syn::Expr::Block(b) => check_function_for_unchecked_calls(&b.block, violations),
        syn::Expr::If(i) => {
            check_expr_recursively(&i.cond, violations);
            check_function_for_unchecked_calls(&i.then_branch, violations);
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr_recursively(else_expr, violations);
            }
        }
        syn::Expr::Match(m) => {
            check_expr_recursively(&m.expr, violations);
            for arm in &m.arms {
                check_expr_recursively(&arm.body, violations);
            }
        }
        syn::Expr::MethodCall(m) => {
            check_expr_recursively(&m.receiver, violations);
            for arg in &m.args {
                check_expr_recursively(arg, violations);
            }
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                check_expr_recursively(arg, violations);
            }
        }
        _ => {}
    }
}

fn is_unchecked_external_call(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method_name = m.method.to_string();
            // Check for common external call patterns
            if method_name == "invoke_contract"
                || method_name == "try_invoke_contract"
                || method_name == "invoke"
            {
                return true;
            }
            // Check if receiver looks like an external client
            if receiver_looks_like_external_client(&m.receiver)
                && !method_looks_read_only(&method_name)
            {
                return true;
            }
            false
        }
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_unchecked_invoke_contract() {
        let rule = UncheckedExternalCallRule::new();
        let source = r#"
            impl MyContract {
                pub fn call_external(env: Env, contract_id: Address) {
                    TokenClient::new(&env, &contract_id).transfer(&from, &to, &amount);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "unchecked external call should be flagged"
        );
    }

    #[test]
    fn no_violation_when_result_is_stored() {
        let rule = UncheckedExternalCallRule::new();
        let source = r#"
            impl MyContract {
                pub fn call_external(env: Env, contract_id: Address) {
                    let result = TokenClient::new(&env, &contract_id).transfer(&from, &to, &amount);
                    result.unwrap();
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "checked external call should not be flagged"
        );
    }

    #[test]
    fn read_only_methods_not_flagged() {
        let rule = UncheckedExternalCallRule::new();
        let source = r#"
            impl MyContract {
                pub fn check_balance(env: Env, contract_id: Address) {
                    TokenClient::new(&env, &contract_id).balance(&user);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "read-only methods should not be flagged"
        );
    }
}
