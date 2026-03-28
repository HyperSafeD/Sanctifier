use crate::rules::{Patch, Rule, RuleViolation, Severity};
use syn::visit::{self, Visit};
use syn::{parse_str, File, Local, Pat};

/// Rule that detects unused local variables.
pub struct UnusedVariableRule;

impl UnusedVariableRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for UnusedVariableRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for UnusedVariableRule {
    fn name(&self) -> &str {
        "unused_variable"
    }

    fn description(&self) -> &str {
        "Detects unused local variables in functions and suggests fixes"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = UnusedVariableVisitor::new();
        visitor.visit_file(&file);

        let mut violations = Vec::new();
        for (ident, span) in visitor.unused_locals {
            let line = span.start().line;
            let col = span.start().column;
            let name = ident.to_string();

            let patch = Patch {
                start_line: line,
                start_column: col,
                end_line: line,
                end_column: col + name.len(),
                replacement: format!("_{}", name),
                description: format!("Prefix unused variable '{}' with underscore", name),
            };

            violations.push(
                RuleViolation::new(
                    self.name(),
                    Severity::Warning,
                    format!("Unused local variable: '{}'", name),
                    format!("{}:{}", line, col),
                )
                .with_suggestion(format!("Prefix with underscore: '_{}'", name))
                .with_patches(vec![patch]),
            );
        }
        violations
    }

    fn fix(&self, source: &str) -> Vec<Patch> {
        self.check(source)
            .into_iter()
            .flat_map(|v| v.patches)
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct UnusedVariableVisitor {
    // Current scope locals: (name, span, was_used)
    locals_stack: Vec<Vec<(String, proc_macro2::Span, bool)>>,
    unused_locals: Vec<(String, proc_macro2::Span)>,
}

impl UnusedVariableVisitor {
    fn new() -> Self {
        Self {
            locals_stack: vec![vec![]],
            unused_locals: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.locals_stack.push(vec![]);
    }

    fn exit_scope(&mut self) {
        if let Some(scope) = self.locals_stack.pop() {
            for (name, span, used) in scope {
                if !used && !name.starts_with('_') && name != "env" {
                    self.unused_locals.push((name, span));
                }
            }
        }
    }

    fn mark_used(&mut self, name: &str) {
        for scope in self.locals_stack.iter_mut().rev() {
            for (n, _, used) in scope.iter_mut() {
                if n == name {
                    *used = true;
                    return;
                }
            }
        }
    }

    fn add_local(&mut self, name: String, span: proc_macro2::Span) {
        if let Some(scope) = self.locals_stack.last_mut() {
            scope.push((name, span, false));
        }
    }
}

impl<'ast> Visit<'ast> for UnusedVariableVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.enter_scope();
        // Skip arguments for now, focus on locals within the body
        visit::visit_item_fn(self, node);
        self.exit_scope();
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.enter_scope();
        visit::visit_impl_item_fn(self, node);
        self.exit_scope();
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.enter_scope();
        visit::visit_block(self, node);
        self.exit_scope();
    }

    fn visit_local(&mut self, node: &'ast Local) {
        if let Pat::Ident(pat_ident) = &node.pat {
            let name = pat_ident.ident.to_string();
            self.add_local(name, pat_ident.ident.span());
        }
        visit::visit_local(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if let Some(ident) = node.path.get_ident() {
            self.mark_used(&ident.to_string());
        }
        visit::visit_expr_path(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_declared_but_unused_local_variable() {
        let rule = UnusedVariableRule::new();
        let source = r#"
            fn compute() -> u32 {
                let result = 42;
                0
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty(), "unused local must be flagged");
        assert!(violations[0].message.contains("result"));
    }

    #[test]
    fn no_violation_when_variable_is_used() {
        let rule = UnusedVariableRule::new();
        let source = r#"
            fn compute() -> u32 {
                let result = 42;
                result
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty(), "used variable must not be flagged");
    }

    #[test]
    fn empty_source_produces_no_findings() {
        let rule = UnusedVariableRule::new();
        let violations = rule.check("");
        assert!(
            violations.is_empty(),
            "empty source must produce no findings"
        );
    }

    #[test]
    fn underscore_prefixed_variable_not_flagged() {
        let rule = UnusedVariableRule::new();
        let source = r#"
            fn compute() {
                let _ignored = expensive_operation();
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "underscore-prefixed variable must not be flagged"
        );
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = UnusedVariableRule::new();
        let violations = rule.check("not valid rust }{{{");
        assert!(
            violations.is_empty(),
            "parse error must return empty, not panic"
        );
    }
}
