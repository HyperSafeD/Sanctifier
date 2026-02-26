use crate::rules::{Rule, RuleViolation, Severity};
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, File};

pub struct ArithmeticOverflowRule;

impl ArithmeticOverflowRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArithmeticOverflowRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ArithmeticOverflowRule {
    fn name(&self) -> &str {
        "arithmetic_overflow"
    }

    fn description(&self) -> &str {
        "Detects unchecked arithmetic operations that could overflow or underflow"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
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

        visitor
            .issues
            .into_iter()
            .map(|issue| {
                RuleViolation::new(
                    self.name(),
                    Severity::Warning,
                    format!("Unchecked '{}' operation could overflow", issue.operation),
                    issue.location,
                )
                .with_suggestion(issue.suggestion)
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct ArithVisitor {
    issues: Vec<ArithmeticIssue>,
    current_fn: Option<String>,
    seen: HashSet<(String, String)>,
}

#[derive(Debug)]
struct ArithmeticIssue {
    operation: String,
    suggestion: String,
    location: String,
}

impl ArithVisitor {
    fn classify_op(op: &syn::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            syn::BinOp::Add(_) => Some((
                "+",
                "Use .checked_add(rhs) or .saturating_add(rhs) to handle overflow",
            )),
            syn::BinOp::Sub(_) => Some((
                "-",
                "Use .checked_sub(rhs) or .saturating_sub(rhs) to handle underflow",
            )),
            syn::BinOp::Mul(_) => Some((
                "*",
                "Use .checked_mul(rhs) or .saturating_mul(rhs) to handle overflow",
            )),
            syn::BinOp::AddAssign(_) => Some((
                "+=",
                "Replace a += b with a = a.checked_add(b).expect(\"overflow\")",
            )),
            syn::BinOp::SubAssign(_) => Some((
                "-=",
                "Replace a -= b with a = a.checked_sub(b).expect(\"underflow\")",
            )),
            syn::BinOp::MulAssign(_) => Some((
                "*=",
                "Replace a *= b with a = a.checked_mul(b).expect(\"overflow\")",
            )),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for ArithVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if let Some(fn_name) = self.current_fn.clone() {
            if let Some((op_str, suggestion)) = Self::classify_op(&node.op) {
                if !is_string_literal(&node.left) && !is_string_literal(&node.right) {
                    let key = (fn_name.clone(), op_str.to_string());
                    if !self.seen.contains(&key) {
                        self.seen.insert(key);
                        let line = node.left.span().start().line;
                        self.issues.push(ArithmeticIssue {
                            operation: op_str.to_string(),
                            suggestion: suggestion.to_string(),
                            location: format!("{}:{}", fn_name, line),
                        });
                    }
                }
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

fn is_string_literal(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        })
    )
}
