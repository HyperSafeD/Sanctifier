use crate::finding_codes::MISSING_RESERVE_AUTH;
use crate::rules::{Rule, RuleViolation, Severity};
use syn::visit::{self, Visit};
use syn::{parse_str, File, Item};

pub struct ReserveWithdrawalRule;

impl ReserveWithdrawalRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReserveWithdrawalRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ReserveWithdrawalRule {
    fn name(&self) -> &str {
        "reserve_withdrawal"
    }

    fn description(&self) -> &str {
        "Detects missing authorization on reserve/treasury withdrawal"
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
                        if let syn::Visibility::Public(_) = f.vis {
                            let fn_name = f.sig.ident.to_string();
                            let fn_name_lower = fn_name.to_lowercase();

                            let is_reserve_path = (fn_name_lower.contains("reserve")
                                || fn_name_lower.contains("treasury"))
                                && (fn_name_lower.contains("withdraw")
                                    || fn_name_lower.contains("transfer")
                                    || fn_name_lower.contains("claim")
                                    || fn_name_lower.contains("drain"));

                            if is_reserve_path {
                                let mut visitor = AuthAndNonceVisitor {
                                    has_auth: false,
                                    has_nonce: false,
                                };
                                visitor.visit_block(&f.block);

                                if !visitor.has_auth && !visitor.has_nonce {
                                    violations.push(RuleViolation::new(
                                        MISSING_RESERVE_AUTH,
                                        Severity::Error,
                                        format!("Function '{}' moves reserve/treasury funds without strong authorization (missing require_auth or admin+nonce check)", fn_name),
                                        fn_name.clone(),
                                    ).with_suggestion("Add require_auth() or a strict admin+nonce validation".to_string()));
                                }
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

struct AuthAndNonceVisitor {
    has_auth: bool,
    has_nonce: bool,
}

impl<'ast> Visit<'ast> for AuthAndNonceVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method_name = node.method.to_string();
        if method_name == "require_auth" || method_name == "require_auth_for_args" {
            self.has_auth = true;
        }
        if method_name.contains("nonce") {
            self.has_nonce = true;
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*node.func {
            if let Some(segment) = p.path.segments.last() {
                let fn_name = segment.ident.to_string();
                if fn_name == "require_auth" || fn_name == "require_auth_for_args" {
                    self.has_auth = true;
                }
                if fn_name.contains("nonce") {
                    self.has_nonce = true;
                }
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(segment) = node.path.segments.last() {
            let name = segment.ident.to_string();
            if name == "require_auth" || name == "require_auth_for_args" {
                self.has_auth = true;
            }
            if name.contains("nonce") {
                self.has_nonce = true;
            }
        }
        visit::visit_macro(self, node);
    }
}
