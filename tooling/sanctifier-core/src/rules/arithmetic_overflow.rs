use crate::rules::{Rule, RuleViolation, Severity};
use crate::ArithmeticIssue;
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, File};

/// Rule that detects unchecked arithmetic operations.
pub struct ArithmeticOverflowRule;

impl ArithmeticOverflowRule {
    /// Create a new instance.
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
            index_depth: 0,
            test_mod_depth: 0,
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

pub(crate) struct ArithVisitor {
    pub(crate) issues: Vec<ArithmeticIssue>,
    pub(crate) current_fn: Option<String>,
    pub(crate) seen: HashSet<(String, String)>,
    /// When >0 we are inside an array-index expression and skip arithmetic.
    pub(crate) index_depth: u32,
    /// When >0 we are inside a #[cfg(test)] module and skip everything.
    pub(crate) test_mod_depth: u32,
}

// Redundant ArithmeticIssue struct removed

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
    // ── Module-level: skip #[cfg(test)] modules entirely ─────────────────────
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_cfg_test(&node.attrs) {
            self.test_mod_depth += 1;
            syn::visit::visit_item_mod(self, node);
            self.test_mod_depth -= 1;
        } else {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if self.test_mod_depth > 0 || has_test_attr(&node.attrs) {
            return;
        }
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if self.test_mod_depth > 0 || has_test_attr(&node.attrs) {
            return;
        }
        let prev = self.current_fn.take();
        self.current_fn = Some(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    // ── Index expressions: don't flag arithmetic in subscripts ────────────────
    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        // Visit the object expression normally (it may contain calls, etc.)
        self.visit_expr(&node.expr);
        // Increase depth so arithmetic inside the index is suppressed.
        self.index_depth += 1;
        self.visit_expr(&node.index);
        self.index_depth -= 1;
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if self.index_depth == 0 {
            if let Some(fn_name) = self.current_fn.clone() {
                if let Some((op_str, suggestion)) = Self::classify_op(&node.op) {
                    if !is_string_literal(&node.left) && !is_string_literal(&node.right) {
                        let key = (fn_name.clone(), op_str.to_string());
                        if !self.seen.contains(&key) {
                            self.seen.insert(key);
                            let line = node.left.span().start().line;
                            self.issues.push(ArithmeticIssue {
                                function_name: fn_name.clone(),
                                operation: op_str.to_string(),
                                suggestion: suggestion.to_string(),
                                location: format!("{}:{}", fn_name, line),
                            });
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if let Some(fn_name) = self.current_fn.clone() {
            let method_name = node.method.to_string();
            if let Some(suggestion) = classify_math_method(&method_name) {
                let key = (fn_name.clone(), method_name.clone());
                if !self.seen.contains(&key) {
                    self.seen.insert(key);
                    let line = node.span().start().line;
                    self.issues.push(ArithmeticIssue {
                        function_name: fn_name.clone(),
                        operation: method_name,
                        suggestion,
                        location: format!("{}:{}", fn_name, line),
                    });
                }
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(fn_name) = self.current_fn.clone() {
            if let syn::Expr::Path(expr_path) = &*node.func {
                if let Some(last_segment) = expr_path.path.segments.last() {
                    let func_name = last_segment.ident.to_string();
                    if let Some(suggestion) = classify_math_call(&func_name) {
                        let key = (fn_name.clone(), func_name.clone());
                        if !self.seen.contains(&key) {
                            self.seen.insert(key);
                            let line = node.span().start().line;
                            self.issues.push(ArithmeticIssue {
                                function_name: fn_name.clone(),
                                operation: func_name,
                                suggestion,
                                location: format!("{}:{}", fn_name, line),
                            });
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

fn classify_math_method(method: &str) -> Option<String> {
    match method {
        "mul_div" => Some("Use '.checked_mul_div()' to handle potential overflow".to_string()),
        "div_ceil" => {
            Some("Consider '.checked_div()' if boundary verification is required".to_string())
        }
        "fixed_point_mul" => Some("Use '.checked_fixed_point_mul()' for safety".to_string()),
        "fixed_point_div" => Some("Use '.checked_fixed_point_div()' for safety".to_string()),
        _ => None,
    }
}

fn classify_math_call(func: &str) -> Option<String> {
    match func {
        "mul_div" => Some("Use 'checked_mul_div' to handle potential overflow".to_string()),
        "fixed_point_mul" => Some("Use 'checked_fixed_point_mul' for safety".to_string()),
        "fixed_point_div" => Some("Use 'checked_fixed_point_div' for safety".to_string()),
        _ => None,
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

/// Returns true if the item has a `#[test]` attribute.
fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("test"))
}

/// Returns true if the item has a `#[cfg(test)]` attribute.
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && quote::quote!(#a).to_string().contains("test"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_standard_arithmetic() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = 1;
                let b = 2;
                let c = a + b;
                let d = a - b;
                let e = a * b;
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn test_flag_custom_math_methods() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = 1;
                let b = 2;
                let c = a.mul_div(5, 10);
                let d = a.fixed_point_mul(b);
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.iter().any(|v| v.message.contains("mul_div")));
        assert!(violations
            .iter()
            .any(|v| v.message.contains("fixed_point_mul")));
    }

    #[test]
    fn test_flag_custom_math_calls() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = mul_div(1, 2, 3);
                let b = fixed_point_div(10, 2);
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.iter().any(|v| v.message.contains("mul_div")));
        assert!(violations
            .iter()
            .any(|v| v.message.contains("fixed_point_div")));
    }

    #[test]
    fn test_ignore_checked_methods() {
        let rule = ArithmeticOverflowRule::new();
        let source = r#"
            fn transfer() {
                let a = 1;
                let b = a.checked_add(2);
                let c = a.checked_mul_div(5, 10);
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_skip_test_attribute_functions() {
        let rule = ArithmeticOverflowRule::new();
        // A #[test] fn with arithmetic should produce zero violations.
        let source = r#"
            #[test]
            fn my_unit_test() {
                let a = 1u64;
                let b = 2u64;
                let c = a + b;
                let d = a - b;
                let e = a * b;
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0, "#[test] fns must be skipped");
    }

    #[test]
    fn test_skip_cfg_test_module() {
        let rule = ArithmeticOverflowRule::new();
        // All arithmetic inside #[cfg(test)] mod must be ignored.
        let source = r#"
            fn mint(amount: u64) {
                let total = amount + 1;
            }

            #[cfg(test)]
            mod tests {
                fn helper() {
                    let x = 1u64 + 2u64;
                    let y = x * 10u64;
                }
            }
        "#;
        let violations = rule.check(source);
        // Only `mint` should fire (1 finding for `+`), not the cfg(test) helper.
        assert_eq!(
            violations.len(),
            1,
            "cfg(test) module arithmetic must be skipped"
        );
    }

    #[test]
    fn test_skip_index_subscript_arithmetic() {
        let rule = ArithmeticOverflowRule::new();
        // i + 1 as an array subscript is idiomatic and should not trigger.
        let source = r#"
            fn read_next(buf: &[u8], i: usize) -> u8 {
                buf[i + 1]
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            0,
            "index subscript arithmetic must be skipped"
        );
    }
}
