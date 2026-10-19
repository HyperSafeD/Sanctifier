use crate::rules::{Patch, Rule, RuleViolation, Severity};
use serde::{Deserialize, Serialize};
use std::path::Path;
use syn::spanned::Spanned;
use syn::{parse_str, File};

/// YAML-based custom rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlCustomRule {
    /// Rule identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Severity level
    pub severity: YamlSeverity,
    /// AST matcher configuration
    pub matcher: AstMatcher,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// Severity values accepted in YAML-defined custom rules.
pub enum YamlSeverity {
    /// Emit an error-level violation.
    Error,
    /// Emit a warning-level violation.
    Warning,
    /// Emit an informational violation.
    Info,
}

impl From<YamlSeverity> for Severity {
    fn from(s: YamlSeverity) -> Self {
        match s {
            YamlSeverity::Error => Severity::Error,
            YamlSeverity::Warning => Severity::Warning,
            YamlSeverity::Info => Severity::Info,
        }
    }
}

/// AST matcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AstMatcher {
    /// Match function calls by name
    #[serde(rename = "function_call")]
    FunctionCall {
        /// Function name pattern (supports wildcards)
        name: String,
        /// Optional: require specific arguments
        #[serde(default)]
        args: Vec<String>,
    },
    /// Match method calls
    #[serde(rename = "method_call")]
    MethodCall {
        /// Method name pattern
        method: String,
        /// Optional: receiver type pattern
        #[serde(default)]
        receiver: Option<String>,
    },
    /// Match storage operations
    #[serde(rename = "storage_operation")]
    StorageOperation {
        /// Operation type: set, get, remove, etc.
        operation: String,
        /// Optional: key pattern
        #[serde(default)]
        key_pattern: Option<String>,
    },
    /// Match by regex pattern (fallback)
    #[serde(rename = "regex")]
    Regex {
        /// Regex pattern
        pattern: String,
    },
}

/// Load custom rules from YAML file
pub fn load_yaml_rules(path: &Path) -> Result<Vec<YamlCustomRule>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read YAML file: {}", e))?;

    serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))
}

/// Wrapper that implements Rule trait for YAML-defined rules
pub struct YamlRuleWrapper {
    rule: YamlCustomRule,
}

impl YamlRuleWrapper {
    /// Create a rule wrapper from a parsed YAML custom rule.
    pub fn new(rule: YamlCustomRule) -> Self {
        Self { rule }
    }
}

impl Rule for YamlRuleWrapper {
    #[allow(clippy::misnamed_getters)]
    fn name(&self) -> &str {
        &self.rule.id
    }

    fn description(&self) -> &str {
        &self.rule.description
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        match &self.rule.matcher {
            AstMatcher::FunctionCall { name, .. } => check_function_calls(source, name, &self.rule),
            AstMatcher::MethodCall { method, receiver } => {
                check_method_calls(source, method, receiver.as_deref(), &self.rule)
            }
            AstMatcher::StorageOperation {
                operation,
                key_pattern,
            } => check_storage_operations(source, operation, key_pattern.as_deref(), &self.rule),
            AstMatcher::Regex { pattern } => check_regex_pattern(source, pattern, &self.rule),
        }
    }

    fn fix(&self, _source: &str) -> Vec<Patch> {
        vec![]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn check_function_calls(
    source: &str,
    name_pattern: &str,
    rule: &YamlCustomRule,
) -> Vec<RuleViolation> {
    let file = match parse_str::<File>(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut violations = Vec::new();
    let visitor = FunctionCallVisitor::new(name_pattern);

    for item in &file.items {
        if let syn::Item::Impl(impl_item) = item {
            for impl_item_inner in &impl_item.items {
                if let syn::ImplItem::Fn(func) = impl_item_inner {
                    visitor.check_block(&func.block, &mut violations, rule);
                }
            }
        }
    }

    violations
}

fn check_method_calls(
    source: &str,
    method_pattern: &str,
    receiver_pattern: Option<&str>,
    rule: &YamlCustomRule,
) -> Vec<RuleViolation> {
    let file = match parse_str::<File>(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut violations = Vec::new();

    for item in &file.items {
        if let syn::Item::Impl(impl_item) = item {
            for impl_item_inner in &impl_item.items {
                if let syn::ImplItem::Fn(func) = impl_item_inner {
                    check_block_for_method_calls(
                        &func.block,
                        method_pattern,
                        receiver_pattern,
                        &mut violations,
                        rule,
                    );
                }
            }
        }
    }

    violations
}

fn check_storage_operations(
    source: &str,
    operation: &str,
    key_pattern: Option<&str>,
    rule: &YamlCustomRule,
) -> Vec<RuleViolation> {
    let file = match parse_str::<File>(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut violations = Vec::new();

    for item in &file.items {
        if let syn::Item::Impl(impl_item) = item {
            for impl_item_inner in &impl_item.items {
                if let syn::ImplItem::Fn(func) = impl_item_inner {
                    check_block_for_storage_ops(
                        &func.block,
                        operation,
                        key_pattern,
                        &mut violations,
                        rule,
                    );
                }
            }
        }
    }

    violations
}

fn check_regex_pattern(source: &str, pattern: &str, rule: &YamlCustomRule) -> Vec<RuleViolation> {
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut violations = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        if re.is_match(line) {
            violations.push(RuleViolation::new(
                &rule.id,
                rule.severity.clone().into(),
                rule.description.clone(),
                format!("line {}", line_num + 1),
            ));
        }
    }

    violations
}

struct FunctionCallVisitor<'a> {
    name_pattern: &'a str,
}

impl<'a> FunctionCallVisitor<'a> {
    fn new(name_pattern: &'a str) -> Self {
        Self { name_pattern }
    }

    fn check_block(
        &self,
        block: &syn::Block,
        violations: &mut Vec<RuleViolation>,
        rule: &YamlCustomRule,
    ) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => {
                    self.check_expr(expr, violations, rule);
                }
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.check_expr(&init.expr, violations, rule);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_expr(
        &self,
        expr: &syn::Expr,
        violations: &mut Vec<RuleViolation>,
        rule: &YamlCustomRule,
    ) {
        match expr {
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path) = &*call.func {
                    if let Some(segment) = path.path.segments.last() {
                        let fn_name = segment.ident.to_string();
                        if matches_pattern(&fn_name, self.name_pattern) {
                            let span = call.span();
                            violations.push(RuleViolation::new(
                                &rule.id,
                                rule.severity.clone().into(),
                                format!("{}: {}", rule.description, fn_name),
                                format!("line {}", span.start().line),
                            ));
                        }
                    }
                }
                for arg in &call.args {
                    self.check_expr(arg, violations, rule);
                }
            }
            syn::Expr::Block(b) => self.check_block(&b.block, violations, rule),
            syn::Expr::If(i) => {
                self.check_expr(&i.cond, violations, rule);
                self.check_block(&i.then_branch, violations, rule);
                if let Some((_, else_expr)) = &i.else_branch {
                    self.check_expr(else_expr, violations, rule);
                }
            }
            syn::Expr::Match(m) => {
                self.check_expr(&m.expr, violations, rule);
                for arm in &m.arms {
                    self.check_expr(&arm.body, violations, rule);
                }
            }
            _ => {}
        }
    }
}

fn check_block_for_method_calls(
    block: &syn::Block,
    method_pattern: &str,
    receiver_pattern: Option<&str>,
    violations: &mut Vec<RuleViolation>,
    rule: &YamlCustomRule,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                check_expr_for_method_calls(
                    expr,
                    method_pattern,
                    receiver_pattern,
                    violations,
                    rule,
                );
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr_for_method_calls(
                        &init.expr,
                        method_pattern,
                        receiver_pattern,
                        violations,
                        rule,
                    );
                }
            }
            _ => {}
        }
    }
}

fn check_expr_for_method_calls(
    expr: &syn::Expr,
    method_pattern: &str,
    receiver_pattern: Option<&str>,
    violations: &mut Vec<RuleViolation>,
    rule: &YamlCustomRule,
) {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method_name = m.method.to_string();
            if matches_pattern(&method_name, method_pattern) {
                let receiver_str = quote::quote!(#m.receiver).to_string();
                let receiver_matches = receiver_pattern
                    .map(|p| matches_pattern(&receiver_str, p))
                    .unwrap_or(true);

                if receiver_matches {
                    let span = m.span();
                    violations.push(RuleViolation::new(
                        &rule.id,
                        rule.severity.clone().into(),
                        format!("{}: {}", rule.description, method_name),
                        format!("line {}", span.start().line),
                    ));
                }
            }
            check_expr_for_method_calls(
                &m.receiver,
                method_pattern,
                receiver_pattern,
                violations,
                rule,
            );
            for arg in &m.args {
                check_expr_for_method_calls(
                    arg,
                    method_pattern,
                    receiver_pattern,
                    violations,
                    rule,
                );
            }
        }
        syn::Expr::Block(b) => {
            check_block_for_method_calls(
                &b.block,
                method_pattern,
                receiver_pattern,
                violations,
                rule,
            );
        }
        syn::Expr::If(i) => {
            check_expr_for_method_calls(
                &i.cond,
                method_pattern,
                receiver_pattern,
                violations,
                rule,
            );
            check_block_for_method_calls(
                &i.then_branch,
                method_pattern,
                receiver_pattern,
                violations,
                rule,
            );
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr_for_method_calls(
                    else_expr,
                    method_pattern,
                    receiver_pattern,
                    violations,
                    rule,
                );
            }
        }
        syn::Expr::Match(m) => {
            check_expr_for_method_calls(
                &m.expr,
                method_pattern,
                receiver_pattern,
                violations,
                rule,
            );
            for arm in &m.arms {
                check_expr_for_method_calls(
                    &arm.body,
                    method_pattern,
                    receiver_pattern,
                    violations,
                    rule,
                );
            }
        }
        _ => {}
    }
}

fn check_block_for_storage_ops(
    block: &syn::Block,
    operation: &str,
    key_pattern: Option<&str>,
    violations: &mut Vec<RuleViolation>,
    rule: &YamlCustomRule,
) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                check_expr_for_storage_ops(expr, operation, key_pattern, violations, rule);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    check_expr_for_storage_ops(
                        &init.expr,
                        operation,
                        key_pattern,
                        violations,
                        rule,
                    );
                }
            }
            _ => {}
        }
    }
}

fn check_expr_for_storage_ops(
    expr: &syn::Expr,
    operation: &str,
    key_pattern: Option<&str>,
    violations: &mut Vec<RuleViolation>,
    rule: &YamlCustomRule,
) {
    match expr {
        syn::Expr::MethodCall(m) => {
            let method_name = m.method.to_string();
            let receiver_str = quote::quote!(#m.receiver).to_string();

            if receiver_str.contains("storage") && method_name == operation {
                let key_matches = if let Some(pattern) = key_pattern {
                    m.args.iter().any(|arg| {
                        let arg_str = quote::quote!(#arg).to_string();
                        matches_pattern(&arg_str, pattern)
                    })
                } else {
                    true
                };

                if key_matches {
                    let span = m.span();
                    violations.push(RuleViolation::new(
                        &rule.id,
                        rule.severity.clone().into(),
                        format!("{}: storage.{}", rule.description, method_name),
                        format!("line {}", span.start().line),
                    ));
                }
            }

            check_expr_for_storage_ops(&m.receiver, operation, key_pattern, violations, rule);
            for arg in &m.args {
                check_expr_for_storage_ops(arg, operation, key_pattern, violations, rule);
            }
        }
        syn::Expr::Block(b) => {
            check_block_for_storage_ops(&b.block, operation, key_pattern, violations, rule);
        }
        syn::Expr::If(i) => {
            check_expr_for_storage_ops(&i.cond, operation, key_pattern, violations, rule);
            check_block_for_storage_ops(&i.then_branch, operation, key_pattern, violations, rule);
            if let Some((_, else_expr)) = &i.else_branch {
                check_expr_for_storage_ops(else_expr, operation, key_pattern, violations, rule);
            }
        }
        syn::Expr::Match(m) => {
            check_expr_for_storage_ops(&m.expr, operation, key_pattern, violations, rule);
            for arm in &m.arms {
                check_expr_for_storage_ops(&arm.body, operation, key_pattern, violations, rule);
            }
        }
        _ => {}
    }
}

fn matches_pattern(text: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            text.contains(pattern.trim_matches('*'))
        }
    } else {
        text.contains(pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("transfer", "transfer"));
        assert!(!matches_pattern("transfer", "mint"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern("unsafe_transfer", "*transfer"));
        assert!(matches_pattern("transfer_from", "transfer*"));
        assert!(matches_pattern("do_transfer_now", "*transfer*"));
    }

    #[test]
    fn test_yaml_rule_function_call() {
        let rule = YamlCustomRule {
            id: "no_unsafe_transfer".to_string(),
            name: "No Unsafe Transfer".to_string(),
            description: "Avoid using unsafe_transfer".to_string(),
            severity: YamlSeverity::Error,
            matcher: AstMatcher::FunctionCall {
                name: "unsafe_transfer".to_string(),
                args: vec![],
            },
        };

        let wrapper = YamlRuleWrapper::new(rule);
        let source = r#"
            impl MyContract {
                pub fn do_transfer(env: Env) {
                    unsafe_transfer(&env, &from, &to);
                }
            }
        "#;

        let violations = wrapper.check(source);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_yaml_rule_method_call() {
        let rule = YamlCustomRule {
            id: "no_direct_remove".to_string(),
            name: "No Direct Remove".to_string(),
            description: "Use safe_remove instead of direct remove".to_string(),
            severity: YamlSeverity::Warning,
            matcher: AstMatcher::MethodCall {
                method: "remove".to_string(),
                receiver: Some("storage".to_string()),
            },
        };

        let wrapper = YamlRuleWrapper::new(rule);
        let source = r#"
            impl MyContract {
                pub fn delete_data(env: Env, key: Symbol) {
                    env.storage().persistent().remove(&key);
                }
            }
        "#;

        let violations = wrapper.check(source);
        assert!(!violations.is_empty());
    }
}
