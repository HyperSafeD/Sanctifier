use crate::rules::{Patch, Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

/// Rule that detects privileged state changes without event emission.
pub struct MissingStateEventRule;

impl MissingStateEventRule {
    /// Create a new missing-state-event rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MissingStateEventRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for MissingStateEventRule {
    fn name(&self) -> &str {
        "missing_state_event"
    }

    fn description(&self) -> &str {
        "Detects privileged state changes (admin, pause, upgrade) without event emission"
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
                        let fn_name = f.sig.ident.to_string();
                        if is_privileged_function(&fn_name) {
                            let mut has_privileged_mutation = false;
                            let mut has_event_emission = false;

                            check_function_body(
                                &f.block,
                                &mut has_privileged_mutation,
                                &mut has_event_emission,
                            );

                            if has_privileged_mutation && !has_event_emission {
                                let span = f.sig.span();
                                violations.push(
                                    RuleViolation::new(
                                        self.name(),
                                        Severity::Warning,
                                        format!(
                                            "Function '{}' changes privileged state without emitting an event",
                                            fn_name
                                        ),
                                        format!("line {}", span.start().line),
                                    )
                                    .with_suggestion(
                                        "Add env.events().publish() after state changes for off-chain observability".to_string()
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
        violations
    }

    fn fix(&self, _source: &str) -> Vec<Patch> {
        // Auto-fix requires context about event structure
        vec![]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn is_privileged_function(fn_name: &str) -> bool {
    let lower = fn_name.to_lowercase();
    lower.contains("admin")
        || lower.contains("owner")
        || lower.contains("pause")
        || lower.contains("unpause")
        || lower.contains("upgrade")
        || lower.contains("set_auth")
        || lower.contains("transfer_ownership")
}

fn check_function_body(
    block: &syn::Block,
    has_privileged_mutation: &mut bool,
    has_event_emission: &mut bool,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                check_expr(expr, has_privileged_mutation, has_event_emission);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr(&init.expr, has_privileged_mutation, has_event_emission);
                }
            }
            _ => {}
        }
    }
}

fn check_expr(expr: &syn::Expr, has_privileged_mutation: &mut bool, has_event_emission: &mut bool) {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method_name = m.method.to_string();

            // Check for storage mutations on privileged keys
            if method_name == "set" || method_name == "update" || method_name == "remove" {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                if receiver_str.contains("storage") {
                    // Check if any argument looks like a privileged key
                    for arg in &m.args {
                        let arg_str = quote::quote!(#arg).to_string();
                        if is_privileged_key(&arg_str) {
                            *has_privileged_mutation = true;
                        }
                    }
                }
            }

            // Check for event emission
            if method_name == "publish" {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                if receiver_str.contains("events") {
                    *has_event_emission = true;
                }
            }

            check_expr(&m.receiver, has_privileged_mutation, has_event_emission);
            for arg in &m.args {
                check_expr(arg, has_privileged_mutation, has_event_emission);
            }
        }
        syn::Expr::Block(b) => {
            check_function_body(&b.block, has_privileged_mutation, has_event_emission);
        }
        syn::Expr::If(i) => {
            check_expr(&i.cond, has_privileged_mutation, has_event_emission);
            check_function_body(&i.then_branch, has_privileged_mutation, has_event_emission);
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr(else_expr, has_privileged_mutation, has_event_emission);
            }
        }
        syn::Expr::Match(m) => {
            check_expr(&m.expr, has_privileged_mutation, has_event_emission);
            for arm in &m.arms {
                check_expr(&arm.body, has_privileged_mutation, has_event_emission);
            }
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                check_expr(arg, has_privileged_mutation, has_event_emission);
            }
        }
        _ => {}
    }
}

fn is_privileged_key(key_str: &str) -> bool {
    let lower = key_str.to_lowercase();
    lower.contains("admin")
        || lower.contains("owner")
        || lower.contains("pause")
        || lower.contains("upgrade")
        || lower.contains("auth")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_admin_change_without_event() {
        let rule = MissingStateEventRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_admin(env: Env, new_admin: Address) {
                    env.storage().persistent().set(&symbol_short!("admin"), &new_admin);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "admin change without event should be flagged"
        );
    }

    #[test]
    fn no_violation_when_event_emitted() {
        let rule = MissingStateEventRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_admin(env: Env, new_admin: Address) {
                    env.storage().persistent().set(&symbol_short!("admin"), &new_admin);
                    env.events().publish(("admin_changed",), new_admin);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "admin change with event should not be flagged"
        );
    }

    #[test]
    fn non_privileged_function_not_flagged() {
        let rule = MissingStateEventRule::new();
        let source = r#"
            impl MyContract {
                pub fn set_balance(env: Env, user: Address, amount: i128) {
                    env.storage().persistent().set(&user, &amount);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "non-privileged function should not be flagged"
        );
    }
}
