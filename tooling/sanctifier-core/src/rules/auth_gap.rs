use crate::rules::{Rule, RuleViolation, Severity};
use syn::visit::Visit;
use syn::{parse_str, File, Item};

pub struct AuthGapRule;

impl AuthGapRule {
    pub fn new() -> Self {
        Self
    }
}

impl Rule for AuthGapRule {
    fn name(&self) -> &str {
        "auth_gap"
    }

    fn description(&self) -> &str {
        "Detects public functions that perform storage mutations without authentication checks"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
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
                            let mut has_read = false;
                            let mut has_auth = false;
                            check_fn_body(
                                &f.block,
                                &mut has_mutation,
                                &mut has_read,
                                &mut has_auth,
                            );
                            if has_mutation && !has_read && !has_auth {
                                gaps.push(RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!("Function '{}' performs storage mutation without authentication", fn_name),
                                    fn_name.clone(),
                                ).with_suggestion("Add require_auth() or require_auth_for_args() before storage operations".to_string()));
                            }
                        }
                    }
                }
            }
        }
        gaps
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn check_fn_body(
    block: &syn::Block,
    has_mutation: &mut bool,
    has_read: &mut bool,
    has_auth: &mut bool,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => check_expr(expr, has_mutation, has_read, has_auth),
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr(&init.expr, has_mutation, has_read, has_auth);
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

fn check_expr(expr: &syn::Expr, has_mutation: &mut bool, has_read: &mut bool, has_auth: &mut bool) {
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
                check_expr(arg, has_mutation, has_read, has_auth);
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
            if method_name == "get" {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                if receiver_str.contains("storage")
                    || receiver_str.contains("persistent")
                    || receiver_str.contains("temporary")
                    || receiver_str.contains("instance")
                {
                    *has_read = true;
                }
            }
            if method_name == "require_auth" || method_name == "require_auth_for_args" {
                *has_auth = true;
            }
            check_expr(&m.receiver, has_mutation, has_read, has_auth);
            for arg in &m.args {
                check_expr(arg, has_mutation, has_read, has_auth);
            }
        }
        syn::Expr::Block(b) => check_fn_body(&b.block, has_mutation, has_read, has_auth),
        syn::Expr::If(i) => {
            check_expr(&i.cond, has_mutation, has_read, has_auth);
            check_fn_body(&i.then_branch, has_mutation, has_read, has_auth);
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr(else_expr, has_mutation, has_read, has_auth);
            }
        }
        syn::Expr::Match(m) => {
            check_expr(&m.expr, has_mutation, has_read, has_auth);
            for arm in &m.arms {
                check_expr(&arm.body, has_mutation, has_read, has_auth);
            }
        }
        _ => {}
    }
}
