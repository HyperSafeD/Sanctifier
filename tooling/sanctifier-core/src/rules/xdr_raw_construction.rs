use crate::rules::{Rule, RuleViolation, Severity};
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{parse_str, Expr, ExprCall, ExprPath, File};

/// Rule that detects direct `xdr::ScVal` raw construction.
pub struct XdrRawConstructionRule;

impl XdrRawConstructionRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for XdrRawConstructionRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for XdrRawConstructionRule {
    fn name(&self) -> &str {
        "xdr_raw_construction"
    }

    fn description(&self) -> &str {
        "Detects direct xdr::ScVal construction and suggests typed ScVal helpers instead"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = XdrRawConstructionVisitor::new();
        visitor.visit_file(&file);

        visitor
            .occurrences
            .into_iter()
            .map(|(line, column)| {
                RuleViolation::new(
                    self.name(),
                    Severity::Info,
                    "Direct xdr::ScVal raw construction is discouraged".to_string(),
                    format!("{}:{}", line, column),
                )
                .with_suggestion("Use typed Soroban helpers or ScVal constructors such as `ScVal::from_u64(...)` instead of raw `xdr::ScVal::...` calls.".to_string())
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct XdrRawConstructionVisitor {
    occurrences: Vec<(usize, usize)>,
    seen: HashSet<(usize, usize)>,
}

impl XdrRawConstructionVisitor {
    fn new() -> Self {
        Self {
            occurrences: Vec::new(),
            seen: HashSet::new(),
        }
    }

    fn record(&mut self, span: proc_macro2::Span) {
        let start = span.start();
        let key = (start.line, start.column);
        if self.seen.insert(key) {
            self.occurrences.push(key);
        }
    }
}

impl<'ast> Visit<'ast> for XdrRawConstructionVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            if is_xdr_scval_constructor(path) {
                self.record(path.path.span());
            }
        }

        visit::visit_expr_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast ExprPath) {
        if is_xdr_scval_constructor(node) {
            self.record(node.path.span());
        }

        visit::visit_expr_path(self, node);
    }
}

fn is_xdr_scval_constructor(expr_path: &ExprPath) -> bool {
    let segments: Vec<_> = expr_path.path.segments.iter().collect();
    if segments.len() < 3 {
        return false;
    }

    segments[0].ident == "xdr" && segments[1].ident == "ScVal"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn load_fixture(name: &str) -> String {
        let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixture_path.push("../../contracts/fixtures");
        fixture_path.push(name);
        fs::read_to_string(&fixture_path).expect("Failed to read fixture")
    }

    #[test]
    fn flags_direct_xdr_scval_raw_construction() {
        let rule = XdrRawConstructionRule::new();
        let source = load_fixture("xdr_raw_construction.rs");

        let violations = rule.check(&source);

        assert_eq!(
            violations.len(),
            2,
            "Expected two direct xdr::ScVal constructions"
        );
        assert_eq!(violations[0].severity, Severity::Info);
        assert!(violations[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("ScVal::from_u64"));
    }

    #[test]
    fn no_findings_for_non_xdr_code() {
        let rule = XdrRawConstructionRule::new();
        let source = r#"
            fn example() {
                let x = 42;
            }
        "#;

        assert!(rule.check(source).is_empty());
    }
}
