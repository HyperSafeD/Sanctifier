use crate::rules::{Rule, RuleViolation, Severity};
use std::collections::HashMap;
use syn::visit::{self, Visit};
use syn::{parse_str, File, Local, Pat};

/// Rule that detects variable shadowing in nested scopes.
///
/// Variable shadowing occurs when a variable declared in an inner scope
/// has the same name as a variable in an outer scope. This can lead to
/// logic bugs where the wrong variable is updated or accessed.
pub struct VariableShadowingRule;

impl VariableShadowingRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for VariableShadowingRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for VariableShadowingRule {
    fn name(&self) -> &str {
        "variable_shadowing"
    }

    fn description(&self) -> &str {
        "Detects variable shadowing in nested scopes that can lead to logic bugs"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = ShadowingVisitor::new();
        visitor.visit_file(&file);

        visitor
            .shadowing_violations
            .into_iter()
            .map(|(name, inner_span, outer_span)| {
                let inner_line = inner_span.start().line;
                let inner_col = inner_span.start().column;
                let outer_line = outer_span.start().line;

                RuleViolation::new(
                    self.name(),
                    Severity::Warning,
                    format!(
                        "Variable '{}' shadows an outer variable declared at line {}",
                        name, outer_line
                    ),
                    format!("{}:{}", inner_line, inner_col),
                )
                .with_suggestion(format!(
                    "Consider renaming the inner variable to avoid shadowing (e.g., '{}_inner', '{}_2')",
                    name, name
                ))
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Visitor that tracks variable declarations across nested scopes.
struct ShadowingVisitor {
    /// Stack of scopes, each containing a map of variable names to their spans.
    /// The last element is the current (innermost) scope.
    scope_stack: Vec<HashMap<String, proc_macro2::Span>>,
    /// Detected shadowing violations: (name, inner_span, outer_span)
    shadowing_violations: Vec<(String, proc_macro2::Span, proc_macro2::Span)>,
    /// Current scope depth (for debugging)
    depth: usize,
}

impl ShadowingVisitor {
    fn new() -> Self {
        Self {
            scope_stack: vec![HashMap::new()],
            shadowing_violations: Vec::new(),
            depth: 0,
        }
    }

    /// Enter a new scope.
    fn enter_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
        self.depth += 1;
    }

    /// Exit the current scope.
    fn exit_scope(&mut self) {
        self.scope_stack.pop();
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Add a variable declaration to the current scope.
    /// If the variable shadows an outer variable, record a violation.
    fn add_variable(&mut self, name: String, span: proc_macro2::Span) {
        // Skip underscore-prefixed variables (intentionally unused)
        if name.starts_with('_') {
            return;
        }

        // Skip common parameter names that are often reused
        if matches!(name.as_str(), "env" | "e" | "self") {
            return;
        }

        // Check if this variable shadows an outer variable
        for outer_scope in self.scope_stack.iter().rev().skip(1) {
            if let Some(&outer_span) = outer_scope.get(&name) {
                self.shadowing_violations
                    .push((name.clone(), span, outer_span));
                break;
            }
        }

        // Add to current scope
        if let Some(current_scope) = self.scope_stack.last_mut() {
            current_scope.insert(name, span);
        }
    }
}

impl<'ast> Visit<'ast> for ShadowingVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.enter_scope();

        // Add function parameters to the scope
        for input in &node.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = pat_ident.ident.to_string();
                    if let Some(current_scope) = self.scope_stack.last_mut() {
                        current_scope.insert(name, pat_ident.ident.span());
                    }
                }
            }
        }

        visit::visit_item_fn(self, node);
        self.exit_scope();
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.enter_scope();

        // Add function parameters to the scope
        for input in &node.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = pat_ident.ident.to_string();
                    if let Some(current_scope) = self.scope_stack.last_mut() {
                        current_scope.insert(name, pat_ident.ident.span());
                    }
                }
            }
        }

        visit::visit_impl_item_fn(self, node);
        self.exit_scope();
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.enter_scope();
        visit::visit_block(self, node);
        self.exit_scope();
    }

    fn visit_local(&mut self, node: &'ast Local) {
        // Extract variable name from pattern
        match &node.pat {
            Pat::Ident(pat_ident) => {
                let name = pat_ident.ident.to_string();
                self.add_variable(name, pat_ident.ident.span());
            }
            Pat::Tuple(pat_tuple) => {
                // Handle tuple destructuring: let (a, b) = ...
                for elem in &pat_tuple.elems {
                    if let Pat::Ident(pat_ident) = elem {
                        let name = pat_ident.ident.to_string();
                        self.add_variable(name, pat_ident.ident.span());
                    }
                }
            }
            _ => {}
        }

        visit::visit_local(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.enter_scope();

        // Add loop variable to scope
        if let Pat::Ident(pat_ident) = &*node.pat {
            let name = pat_ident.ident.to_string();
            self.add_variable(name, pat_ident.ident.span());
        }

        visit::visit_expr_for_loop(self, node);
        self.exit_scope();
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.enter_scope();

        // Add closure parameters to scope
        for input in &node.inputs {
            if let Pat::Ident(pat_ident) = input {
                let name = pat_ident.ident.to_string();
                self.add_variable(name, pat_ident.ident.span());
            }
        }

        visit::visit_expr_closure(self, node);
        self.exit_scope();
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        // Visit the match expression first
        visit::visit_expr(self, &node.expr);

        // Visit each arm - each arm gets its own scope for pattern bindings
        for arm in &node.arms {
            self.enter_scope();

            // Extract pattern bindings and check for shadowing
            extract_pattern_bindings(&arm.pat, self);

            // Visit the guard if present
            if let Some((_, guard)) = &arm.guard {
                visit::visit_expr(self, guard);
            }

            // Visit the arm body
            visit::visit_expr(self, &arm.body);

            self.exit_scope();
        }
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        // Don't do anything here - we handle arms in visit_expr_match
        // to ensure proper scoping
        visit::visit_arm(self, node);
    }
}

/// Extract variable bindings from a pattern (used in match arms).
fn extract_pattern_bindings(pat: &Pat, visitor: &mut ShadowingVisitor) {
    match pat {
        Pat::Ident(pat_ident) => {
            let name = pat_ident.ident.to_string();
            visitor.add_variable(name, pat_ident.ident.span());
        }
        Pat::Tuple(pat_tuple) => {
            for elem in &pat_tuple.elems {
                extract_pattern_bindings(elem, visitor);
            }
        }
        Pat::Struct(pat_struct) => {
            for field in &pat_struct.fields {
                extract_pattern_bindings(&field.pat, visitor);
            }
        }
        Pat::TupleStruct(pat_tuple_struct) => {
            for elem in &pat_tuple_struct.elems {
                extract_pattern_bindings(elem, visitor);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_simple_shadowing() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn compute() -> u32 {
                let x = 10;
                {
                    let x = 20;  // shadows outer x
                    x
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1, "should detect one shadowing violation");
        assert!(violations[0].message.contains("'x'"));
        assert!(violations[0].message.contains("shadows"));
    }

    #[test]
    fn test_detects_shadowing_in_nested_blocks() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn process() {
                let value = 100;
                if true {
                    let value = 200;  // shadows outer value
                    println!("{}", value);
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("'value'"));
    }

    #[test]
    fn test_detects_shadowing_in_for_loop() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn iterate() {
                let i = 0;
                for i in 0..10 {  // shadows outer i
                    println!("{}", i);
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("'i'"));
    }

    #[test]
    fn test_no_violation_for_different_scopes() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn compute() {
                {
                    let x = 10;
                }
                {
                    let x = 20;  // different scope, not shadowing
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0, "sibling scopes should not trigger");
    }

    #[test]
    fn test_no_violation_for_underscore_prefix() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn compute() {
                let _x = 10;
                {
                    let _x = 20;  // underscore prefix, intentional
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(
            violations.len(),
            0,
            "underscore-prefixed variables should be ignored"
        );
    }

    #[test]
    fn test_detects_shadowing_in_match_arm() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn handle(opt: Option<u32>) {
                let value = 100;
                match opt {
                    Some(value) => println!("{}", value),  // shadows outer value
                    None => {}
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("'value'"));
    }

    #[test]
    fn test_detects_shadowing_in_closure() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn process() {
                let x = 10;
                let closure = |x| {  // shadows outer x
                    x + 1
                };
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("'x'"));
    }

    #[test]
    fn test_no_violation_for_env_parameter() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            impl Contract {
                pub fn transfer(env: Env) {
                    let env = env.clone();  // common pattern, should be ignored
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0, "env parameter should be ignored");
    }

    #[test]
    fn test_multiple_shadowing_violations() {
        let rule = VariableShadowingRule::new();
        let source = r#"
            fn complex() {
                let x = 1;
                let y = 2;
                {
                    let x = 10;  // shadows x
                    let y = 20;  // shadows y
                }
            }
        "#;
        let violations = rule.check(source);
        assert_eq!(violations.len(), 2, "should detect both shadowing cases");
    }

    #[test]
    fn test_invalid_source_produces_no_panic() {
        let rule = VariableShadowingRule::new();
        let violations = rule.check("not valid rust }{{{");
        assert_eq!(
            violations.len(),
            0,
            "parse error must return empty, not panic"
        );
    }
}
