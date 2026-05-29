use crate::rules::{Rule, RuleViolation, Severity};
use quote::ToTokens;
use syn::{parse_str, Expr, File, Item, Stmt};

/// Rule to detect usage of env.storage().instance().update() without a state check.
pub struct StorageUpdateStateCheckRule;

impl StorageUpdateStateCheckRule {
    /// Creates a new instance of the rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for StorageUpdateStateCheckRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for StorageUpdateStateCheckRule {
    fn name(&self) -> &str {
        "storage_update_state_check"
    }

    fn description(&self) -> &str {
        "Detects update() without preceding state check (e.g., has()) to prevent accidental state wipes."
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
                    if let syn::ImplItem::Fn(fn_item) = impl_item {
                        if let syn::Visibility::Public(_) = fn_item.vis {
                            let fn_name = fn_item.sig.ident.to_string();
                            if is_vulnerable_storage_update(&fn_item.block.stmts) {
                                violations.push(RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "Function '{}' calls update() without an explicit state check.",
                                        fn_name
                                    ),
                                    fn_name,
                                ).with_suggestion(
                                    "Ensure there is a state check (e.g., env.storage().instance().has()) before calling update() to prevent accidental overwrites.".to_string()
                                ));
                            }
                        }
                    }
                }
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn is_vulnerable_storage_update(stmts: &[Stmt]) -> bool {
    let mut has_check = false;

    for stmt in stmts {
        if check_for_state_check(stmt) {
            has_check = true;
        }

        if check_for_vulnerable_update(stmt, has_check) {
            return true;
        }
    }

    false
}

fn check_for_state_check(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr, _) => is_storage_has_call(expr),
        Stmt::Local(local) => {
            if let Some(init) = &local.init {
                is_storage_has_call(&init.expr)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn check_for_vulnerable_update(stmt: &Stmt, has_check: bool) -> bool {
    if let Stmt::Expr(expr, _) = stmt {
        if is_storage_update_call(expr) {
            return !has_check;
        }
        if let Expr::If(expr_if) = expr {
            if is_storage_has_call(&expr_if.cond) {
                return false;
            }
            for s in &expr_if.then_branch.stmts {
                if check_for_vulnerable_update(s, has_check) {
                    return true;
                }
            }
            if let Some((_, else_branch)) = &expr_if.else_branch {
                if let Expr::Block(expr_block) = else_branch.as_ref() {
                    for s in &expr_block.block.stmts {
                        if check_for_vulnerable_update(s, has_check) {
                            return true;
                        }
                    }
                }
            }
            if let Expr::Match(expr_match) = expr {
                if is_storage_has_call(&expr_match.expr) {
                    return false;
                }
                for arm in &expr_match.arms {
                    if let Expr::Block(expr_block) = arm.body.as_ref() {
                        for s in &expr_block.block.stmts {
                            if check_for_vulnerable_update(s, has_check) {
                                return true;
                            }
                        }
                    }
                }
            }
            if let Expr::Match(expr_match) = expr {
                if is_storage_has_call(&expr_match.expr) {
                    return false;
                }
                for arm in &expr_match.arms {
                    if let Expr::Block(expr_block) = arm.body.as_ref() {
                        for s in &expr_block.block.stmts {
                            if check_for_vulnerable_update(s, has_check) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_storage_has_call(expr: &Expr) -> bool {
    let s = expr.to_token_stream().to_string().replace(" ", "");
    s.contains("storage().instance().has")
}

fn is_storage_update_call(expr: &Expr) -> bool {
    let s = expr.to_token_stream().to_string().replace(" ", "");
    s.contains("storage().instance().update")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_update_without_check() {
        let source = r#"
            impl MyContract {
                pub fn vulnerable(env: Env) {
                    env.storage().instance().update(&KEY, |old| {
                        Ok(new_value)
                    });
                }
            }
        "#;
        let rule = StorageUpdateStateCheckRule::new();
        let violations = rule.check(source);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_storage_update_with_check() {
        let source = r#"
            impl MyContract {
                pub fn safe(env: Env) {
                    if env.storage().instance().has(&KEY) {
                        env.storage().instance().update(&KEY, |old| {
                            Ok(new_value)
                        });
                    }
                }
            }
        "#;
        let rule = StorageUpdateStateCheckRule::new();
        let violations = rule.check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_storage_update_with_match_check() {
        let source = r#"
            impl MyContract {
                pub fn safe(env: Env) {
                    match env.storage().instance().has(&KEY) {
                        true => {
                            env.storage().instance().update(&KEY, |old| {
                                Ok(new_value)
                            });
                        }
                        false => {}
                    }
                }
            }
        "#;
        let rule = StorageUpdateStateCheckRule::new();
        let violations = rule.check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_storage_update_after_check() {
        let source = r#"
            impl MyContract {
                pub fn safe(env: Env) {
                    let exists = env.storage().instance().has(&KEY);
                    if exists {
                        env.storage().instance().update(&KEY, |old| {
                            Ok(new_value)
                        });
                    }
                }
            }
        "#;
        let rule = StorageUpdateStateCheckRule::new();
        let violations = rule.check(source);
        assert!(violations.is_empty());
    }
}
