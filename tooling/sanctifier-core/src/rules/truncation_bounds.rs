use crate::rules::{Rule, RuleViolation, Severity};
use crate::TruncationBoundsIssue;
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_str, File};

/// Rule that detects integer truncation casts and unchecked array/slice indexing.
pub struct TruncationBoundsRule;

impl TruncationBoundsRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TruncationBoundsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for TruncationBoundsRule {
    fn name(&self) -> &str {
        "truncation_bounds"
    }

    fn description(&self) -> &str {
        "Detects narrowing integer casts (as u32/u16/u8/i32/i16/i8) and unchecked array/slice indexing"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = TruncationBoundsVisitor {
            issues: Vec::new(),
            current_fn: None,
            seen: HashSet::new(),
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
                    format!(
                        "{} risk: `{}`",
                        if issue.kind == "truncation" {
                            "Integer truncation"
                        } else {
                            "Unchecked index"
                        },
                        issue.expression
                    ),
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

pub(crate) struct TruncationBoundsVisitor {
    pub(crate) issues: Vec<TruncationBoundsIssue>,
    pub(crate) current_fn: Option<String>,
    pub(crate) seen: HashSet<(String, String)>,
    /// When >0 we are inside a #[cfg(test)] module and skip everything.
    pub(crate) test_mod_depth: u32,
}

/// Narrowing target types that indicate potential truncation.
const NARROWING_TYPES: &[&str] = &["u32", "u16", "u8", "i32", "i16", "i8"];

impl<'ast> Visit<'ast> for TruncationBoundsVisitor {
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

    // ── Detect `as <narrowing_type>` casts ───────────────────────────────────
    fn visit_expr_cast(&mut self, node: &'ast syn::ExprCast) {
        if let Some(fn_name) = self.current_fn.clone() {
            if let syn::Type::Path(type_path) = &*node.ty {
                if let Some(segment) = type_path.path.segments.last() {
                    let ty_name = segment.ident.to_string();
                    if NARROWING_TYPES.contains(&ty_name.as_str()) {
                        let expr_str = format!("as {}", ty_name);
                        let key = (fn_name.clone(), expr_str.clone());
                        if !self.seen.contains(&key) {
                            self.seen.insert(key);
                            let line = node.span().start().line;
                            self.issues.push(TruncationBoundsIssue {
                                function_name: fn_name.clone(),
                                kind: "truncation".to_string(),
                                expression: expr_str,
                                suggestion: format!(
                                    "Use `{ty_name}::try_from(val).unwrap_or(...)` or \
                                     `.try_into()` with proper error handling instead of `as {ty_name}`"
                                ),
                                location: format!("{}:{}", fn_name, line),
                            });
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_cast(self, node);
    }

    // ── Detect unchecked array/slice indexing ─────────────────────────────────
    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        if let Some(fn_name) = self.current_fn.clone() {
            let expr_str = quote::quote!(#node).to_string();
            // Use a simplified key: fn_name + "index"
            let key = (fn_name.clone(), format!("index:{}", expr_str));
            if !self.seen.contains(&key) {
                self.seen.insert(key);
                let line = node.span().start().line;
                self.issues.push(TruncationBoundsIssue {
                    function_name: fn_name.clone(),
                    kind: "unchecked_index".to_string(),
                    expression: expr_str,
                    suggestion: "Use `.get(index)` with bounds checking instead of direct indexing"
                        .to_string(),
                    location: format!("{}:{}", fn_name, line),
                });
            }
        }
        syn::visit::visit_expr_index(self, node);
    }
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
    fn test_detect_narrowing_cast() {
        let rule = TruncationBoundsRule::new();
        let source = r#"
            fn convert(val: u64) -> u32 {
                val as u32
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("truncation"));
    }

    #[test]
    fn test_detect_unchecked_index() {
        let rule = TruncationBoundsRule::new();
        let source = r#"
            fn read(buf: &[u8], i: usize) -> u8 {
                buf[i]
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("index"));
    }

    #[test]
    fn test_skip_test_functions() {
        let rule = TruncationBoundsRule::new();
        let source = r#"
            #[test]
            fn my_test() {
                let x: u32 = 100u64 as u32;
                let v = vec![1, 2, 3];
                let _ = v[0];
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_skip_cfg_test_module() {
        let rule = TruncationBoundsRule::new();
        let source = r#"
            fn real_code(val: u64) -> u32 {
                val as u32
            }

            #[cfg(test)]
            mod tests {
                fn helper() {
                    let x: u32 = 100u64 as u32;
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            1,
            "Only real_code should fire, not cfg(test) helper"
        );
    }

    #[test]
    fn test_dedup_same_cast_type() {
        let rule = TruncationBoundsRule::new();
        let source = r#"
            fn convert(a: u64, b: u64) -> (u32, u32) {
                (a as u32, b as u32)
            }
        "#;
        let violations = rule.check(source);
        // Same (function, "as u32") pair — deduplicated to 1
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_multiple_cast_types() {
        let rule = TruncationBoundsRule::new();
        let source = r#"
            fn convert(a: u64, b: i64) -> (u32, u16) {
                (a as u32, b as u16)
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 2);
    }
}
