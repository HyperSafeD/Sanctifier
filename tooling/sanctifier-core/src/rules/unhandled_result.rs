use crate::rules::{Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, File, Type};

pub struct UnhandledResultRule;

impl UnhandledResultRule {
    pub fn new() -> Self {
        Self
    }
}

impl Rule for UnhandledResultRule {
    fn name(&self) -> &str {
        "unhandled_result"
    }

    fn description(&self) -> &str {
        "Detects unhandled Result types in public contract functions"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = ResultVisitor {
            issues: Vec::new(),
            current_fn: None,
            is_public_fn: false,
        };
        visitor.visit_file(&file);

        visitor
            .issues
            .into_iter()
            .map(|issue| {
                RuleViolation::new(
                    self.name(),
                    Severity::Warning,
                    issue.message,
                    issue.location,
                )
                .with_suggestion(
                    "Use ?, match, or .unwrap()/.expect() to handle the Result".to_string(),
                )
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct ResultVisitor {
    issues: Vec<UnhandledResultIssue>,
    current_fn: Option<String>,
    is_public_fn: bool,
}

struct UnhandledResultIssue {
    message: String,
    location: String,
}

impl ResultVisitor {
    fn is_result_type(ty: &Type) -> bool {
        if let Type::Path(tp) = ty {
            if let Some(seg) = tp.path.segments.last() {
                return seg.ident == "Result";
            }
        }
        false
    }

    fn is_result_returning_fn(sig: &syn::Signature) -> bool {
        if let syn::ReturnType::Type(_, ty) = &sig.output {
            Self::is_result_type(ty)
        } else {
            false
        }
    }

    fn is_handled(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Try(_) => true,
            syn::Expr::Match(_) => true,
            syn::Expr::MethodCall(m) => {
                let method = m.method.to_string();
                matches!(
                    method.as_str(),
                    "unwrap"
                        | "expect"
                        | "unwrap_or"
                        | "unwrap_or_else"
                        | "unwrap_or_default"
                        | "ok"
                        | "err"
                        | "is_ok"
                        | "is_err"
                        | "map"
                        | "map_err"
                        | "and_then"
                        | "or_else"
                        | "unwrap_unchecked"
                        | "expect_unchecked"
                )
            }
            syn::Expr::Assign(a) => Self::is_handled(&a.right),
            syn::Expr::Call(c) => {
                if let syn::Expr::Path(p) = &*c.func {
                    if let Some(seg) = p.path.segments.last() {
                        if seg.ident == "Ok" || seg.ident == "Err" {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn call_returns_result(call: &syn::ExprCall) -> bool {
        if let syn::Expr::Path(p) = &*call.func {
            if let Some(seg) = p.path.segments.last() {
                let name = seg.ident.to_string();
                return !matches!(name.as_str(), "Ok" | "Err" | "Some" | "None" | "panic");
            }
        }
        false
    }

    fn expr_to_string(expr: &syn::Expr) -> String {
        let s = quote::quote!(#expr).to_string();
        if s.len() > 80 {
            format!("{}...", &s[..77])
        } else {
            s
        }
    }
}

impl<'ast> Visit<'ast> for ResultVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev_fn = self.current_fn.take();
        let prev_public = self.is_public_fn;

        self.current_fn = Some(node.sig.ident.to_string());
        self.is_public_fn = matches!(node.vis, syn::Visibility::Public(_));

        let fn_returns_result = Self::is_result_returning_fn(&node.sig);

        for stmt in &node.block.stmts {
            self.check_statement(stmt, fn_returns_result);
        }

        self.current_fn = prev_fn;
        self.is_public_fn = prev_public;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev_fn = self.current_fn.take();
        let prev_public = self.is_public_fn;

        self.current_fn = Some(node.sig.ident.to_string());
        self.is_public_fn = matches!(node.vis, syn::Visibility::Public(_));

        let fn_returns_result = Self::is_result_returning_fn(&node.sig);

        for stmt in &node.block.stmts {
            self.check_statement(stmt, fn_returns_result);
        }

        self.current_fn = prev_fn;
        self.is_public_fn = prev_public;
    }
}

impl ResultVisitor {
    fn check_statement(&mut self, stmt: &syn::Stmt, fn_returns_result: bool) {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.check_expr(expr, fn_returns_result);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.check_expr(&init.expr, fn_returns_result);
                }
            }
            _ => {}
        }
    }

    fn check_expr(&mut self, expr: &syn::Expr, fn_returns_result: bool) {
        match expr {
            syn::Expr::Call(call) => {
                if Self::is_handled(expr) {
                    return;
                }
                if Self::call_returns_result(call) && !fn_returns_result && self.is_public_fn {
                    if let Some(fn_name) = &self.current_fn {
                        let line = expr.span().start().line;
                        self.issues.push(UnhandledResultIssue {
                            message: format!(
                                "Result returned from '{}' is not handled",
                                Self::expr_to_string(expr)
                            ),
                            location: format!("{}:{}", fn_name, line),
                        });
                    }
                }
                for arg in &call.args {
                    self.check_expr(arg, fn_returns_result);
                }
            }
            syn::Expr::MethodCall(m) => {
                if !Self::is_handled(expr) {
                    self.check_expr(&m.receiver, fn_returns_result);
                }
                for arg in &m.args {
                    self.check_expr(arg, fn_returns_result);
                }
            }
            syn::Expr::Try(e) => {
                self.check_expr(&e.expr, true);
            }
            syn::Expr::Match(m) => {
                for arm in &m.arms {
                    self.check_expr(&arm.body, fn_returns_result);
                }
            }
            syn::Expr::If(i) => {
                self.check_expr(&i.cond, fn_returns_result);
                self.check_block(&i.then_branch, fn_returns_result);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr(else_expr, fn_returns_result);
                }
            }
            syn::Expr::Block(b) => {
                self.check_block(&b.block, fn_returns_result);
            }
            syn::Expr::Assign(a) => {
                self.check_expr(&a.right, fn_returns_result);
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &syn::Block, fn_returns_result: bool) {
        for stmt in &block.stmts {
            self.check_statement(stmt, fn_returns_result);
        }
    }
}
