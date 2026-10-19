use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, Expr, File, Item, Stmt};

/// Rule to detect shadow storage patterns in Soroban contracts.
pub struct ShadowStorageRule;

impl ShadowStorageRule {
    /// Creates a new instance of the shadow storage rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShadowStorageRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ShadowStorageRule {
    fn name(&self) -> &str {
        "shadow_storage"
    }

    fn description(&self) -> &str {
        "Detects functions that modify contract storage without emitting corresponding events, creating transparency gaps for indexers"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                if is_cfg_test_attrs(&impl_block.attrs) {
                    continue;
                }

                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(fn_item) = impl_item {
                        if has_attr(&fn_item.attrs, "test") {
                            continue;
                        }

                        if let syn::Visibility::Public(_) = fn_item.vis {
                            let fn_name = fn_item.sig.ident.to_string();

                            if is_reserved_soroban_entrypoint(&fn_name) {
                                continue;
                            }

                            let mut storage_ops: Vec<String> = Vec::new();
                            let mut events_emitted: Vec<String> = Vec::new();

                            analyze_function_body(
                                &fn_item.block.stmts,
                                &mut storage_ops,
                                &mut events_emitted,
                            );

                            if !storage_ops.is_empty() && events_emitted.is_empty() {
                                let location = fn_name.clone();
                                violations.push(
                                    RuleViolation::new(
                                        self.name(),
                                        Severity::Warning,
                                        format!(
                                            "Function '{}' modifies storage without emitting events. This creates transparency gaps for indexers and off-chain monitors.",
                                            fn_name
                                        ),
                                        location,
                                    ).with_suggestion(
                                        "Emit an event after each storage mutation using env.events().publish() to ensure indexers can track state changes. Example: env.events().publish((symbol_short!(\"event_name\"), key), data)".to_string()
                                    )
                                );
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

fn is_reserved_soroban_entrypoint(fn_name: &str) -> bool {
    matches!(fn_name, "__constructor" | "__check_auth")
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        matches!(&attr.meta, syn::Meta::Path(path) if path.is_ident(name) || path.segments.iter().any(|s| s.ident == name))
    })
}

fn is_cfg_test_attrs(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && quote::quote!(#a).to_string().contains("test"))
}

fn analyze_function_body(
    stmts: &[Stmt],
    storage_ops: &mut Vec<String>,
    events_emitted: &mut Vec<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => {
                analyze_expr(expr, storage_ops, events_emitted);
            }
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    analyze_expr(&init.expr, storage_ops, events_emitted);
                }
            }
            _ => {}
        }
    }
}

fn analyze_expr(expr: &Expr, storage_ops: &mut Vec<String>, events_emitted: &mut Vec<String>) {
    match expr {
        Expr::MethodCall(method_call) => {
            let method_name = method_call.method.to_string();

            if is_storage_mutation_method(&method_name) {
                let receiver_str = quote::quote!(#method_call.receiver).to_string();
                if receiver_str.contains("storage")
                    || receiver_str.contains("persistent")
                    || receiver_str.contains("temporary")
                    || receiver_str.contains("instance")
                {
                    storage_ops.push(method_name.clone());
                }
            }

            if method_name == "publish" || method_name == "emit" {
                let receiver_str = quote::quote!(#method_call.receiver).to_string();
                if receiver_str.contains("events") {
                    events_emitted.push(method_name.clone());
                }
            }

            analyze_expr(&method_call.receiver, storage_ops, events_emitted);
            for arg in &method_call.args {
                analyze_expr(arg, storage_ops, events_emitted);
            }
        }
        Expr::Call(call) => {
            if let Expr::Path(path) = &*call.func {
                if let Some(segment) = path.path.segments.last() {
                    let ident = segment.ident.to_string();
                    if ident == "publish" || ident == "emit" {
                        events_emitted.push(ident);
                    }
                }
            }
            for arg in &call.args {
                analyze_expr(arg, storage_ops, events_emitted);
            }
        }
        Expr::Block(block) => {
            analyze_function_body(&block.block.stmts, storage_ops, events_emitted);
        }
        Expr::If(if_expr) => {
            analyze_expr(&if_expr.cond, storage_ops, events_emitted);
            analyze_function_body(&if_expr.then_branch.stmts, storage_ops, events_emitted);
            if let Some((_, else_expr)) = &if_expr.else_branch {
                analyze_expr(else_expr, storage_ops, events_emitted);
            }
        }
        Expr::Match(match_expr) => {
            analyze_expr(&match_expr.expr, storage_ops, events_emitted);
            for arm in &match_expr.arms {
                analyze_expr(&arm.body, storage_ops, events_emitted);
            }
        }
        Expr::Loop(loop_expr) => {
            analyze_function_body(&loop_expr.body.stmts, storage_ops, events_emitted);
        }
        Expr::ForLoop(for_expr) => {
            analyze_expr(&for_expr.expr, storage_ops, events_emitted);
            analyze_function_body(&for_expr.body.stmts, storage_ops, events_emitted);
        }
        Expr::While(while_expr) => {
            analyze_expr(&while_expr.cond, storage_ops, events_emitted);
            analyze_function_body(&while_expr.body.stmts, storage_ops, events_emitted);
        }
        Expr::Assign(assign) => {
            analyze_expr(&assign.left, storage_ops, events_emitted);
            analyze_expr(&assign.right, storage_ops, events_emitted);
        }
        Expr::Unary(unary) => {
            analyze_expr(&unary.expr, storage_ops, events_emitted);
        }
        Expr::Binary(binary) => {
            analyze_expr(&binary.left, storage_ops, events_emitted);
            analyze_expr(&binary.right, storage_ops, events_emitted);
        }
        Expr::Try(try_expr) => {
            analyze_expr(&try_expr.expr, storage_ops, events_emitted);
        }
        Expr::Await(await_expr) => {
            analyze_expr(&await_expr.base, storage_ops, events_emitted);
        }
        Expr::Closure(closure) => {
            analyze_expr(&closure.body, storage_ops, events_emitted);
        }
        Expr::Tuple(tuple) => {
            for elem in &tuple.elems {
                analyze_expr(elem, storage_ops, events_emitted);
            }
        }
        Expr::Array(array) => {
            for elem in &array.elems {
                analyze_expr(elem, storage_ops, events_emitted);
            }
        }
        Expr::Struct(struct_expr) => {
            for field in &struct_expr.fields {
                analyze_expr(&field.expr, storage_ops, events_emitted);
            }
        }
        Expr::Return(return_expr) => {
            if let Some(returned_expr) = &return_expr.expr {
                analyze_expr(returned_expr, storage_ops, events_emitted);
            }
        }
        Expr::Field(field) => {
            analyze_expr(&field.base, storage_ops, events_emitted);
        }
        Expr::Index(index) => {
            analyze_expr(&index.expr, storage_ops, events_emitted);
            analyze_expr(&index.index, storage_ops, events_emitted);
        }
        Expr::Reference(reference) => {
            analyze_expr(&reference.expr, storage_ops, events_emitted);
        }
        Expr::Paren(paren) => {
            analyze_expr(&paren.expr, storage_ops, events_emitted);
        }
        Expr::Group(group) => {
            analyze_expr(&group.expr, storage_ops, events_emitted);
        }
        Expr::Cast(cast) => {
            analyze_expr(&cast.expr, storage_ops, events_emitted);
        }
        _ => {}
    }
}

fn is_storage_mutation_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "set"
            | "update"
            | "remove"
            | "put"
            | "insert"
            | "delete"
            | "extend"
            | "extend_ttl"
            | "extend_ttl_persistent"
            | "extend_ttl_instance"
    )
}

#[cfg(test)]
mod tests {
    use crate::Analyzer;
    use crate::SanctifyConfig;

    #[test]
    fn test_shadow_storage_detection() {
        let source = r#"
#[contractimpl]
impl MyContract {
    pub fn set_value(env: Env, key: Symbol, value: u32) {
        env.storage().persistent().set(&key, &value);
    }

    pub fn set_value_with_event(env: Env, key: Symbol, value: u32) {
        env.storage().persistent().set(&key, &value);
        env.events().publish((symbol_short!("value_set"), key.clone()), value);
    }

    pub fn get_value(env: Env, key: Symbol) -> u32 {
        env.storage().persistent().get(&key).unwrap_or(0)
    }
}
"#;
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let issues = analyzer.run_rule(source, "shadow_storage");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].rule_name, "shadow_storage");
        assert!(issues[0].message.contains("set_value"));
    }

    #[test]
    fn test_shadow_storage_no_violation_for_event_emit() {
        let source = r#"
#[contractimpl]
impl Token {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        to.require_auth();
        
        let balance_from = env.storage().persistent().get(&from).unwrap_or(0);
        let balance_to = env.storage().persistent().get(&to).unwrap_or(0);
        
        env.storage().persistent().set(&from, &(balance_from - amount));
        env.storage().persistent().set(&to, &(balance_to + amount));
        
        env.events().publish((symbol_short!("transfer"), from, to), amount);
    }
}
"#;
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let issues = analyzer.run_rule(source, "shadow_storage");
        assert!(
            issues.is_empty(),
            "Expected no violations when event is emitted, got: {:?}",
            issues
        );
    }

    #[test]
    fn test_shadow_storage_reserved_entrypoint_ignored() {
        let source = r#"
#[contractimpl]
impl Account {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&symbol_short!("admin"), &admin);
    }
}
"#;
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let issues = analyzer.run_rule(source, "shadow_storage");
        assert!(
            issues.is_empty(),
            "Reserved entrypoint should not be flagged: {:?}",
            issues
        );
    }

    #[test]
    fn test_shadow_storage_private_fn_not_flagged() {
        let source = r#"
impl MyContract {
    fn internal_update(env: Env, key: Symbol, value: u32) {
        env.storage().persistent().set(&key, &value);
    }
}
"#;
        let analyzer = Analyzer::new(SanctifyConfig::default());
        let issues = analyzer.run_rule(source, "shadow_storage");
        assert!(
            issues.is_empty(),
            "Private functions should not be flagged: {:?}",
            issues
        );
    }
}
