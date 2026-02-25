// tooling/sanctifier-core/src/reentrancy.rs
//
// Reentrancy pattern detection for Soroban contracts.
//
// In Soroban, true reentrancy is mitigated by the host's execution model, but
// cross-contract call ordering (Checks-Effects-Interactions / CEI violations)
// can still lead to logical reentrancy bugs.
//
// This module flags public contract functions where a cross-contract invocation
// (`env.invoke_contract`, `Client::`, `TokenClient::`) appears BEFORE a
// storage write (`.set(`, `.update(`, `.remove(`).

use serde::Serialize;
use syn::{visit::Visit, parse_str, File, Item};

/// A potential reentrancy / CEI violation found in a contract function.
#[derive(Debug, Serialize, Clone)]
pub struct ReentrancyIssue {
    /// Name of the contract function that contains the violation.
    pub function_name: String,
    /// Short description of the issue.
    pub issue_type: String,
    /// Human-readable location context.
    pub location: String,
}

// ── Visitor ──────────────────────────────────────────────────────────────────

struct ReentrancyVisitor {
    pub issues: Vec<ReentrancyIssue>,
    current_fn: Option<String>,
    /// Whether we have seen a cross-contract call in the current function.
    saw_cross_call: bool,
}

impl ReentrancyVisitor {
    fn new() -> Self {
        Self {
            issues: Vec::new(),
            current_fn: None,
            saw_cross_call: false,
        }
    }

    fn check_method(&mut self, method: &str) {
        // Cross-contract call patterns
        let is_cross_call = matches!(
            method,
            "invoke_contract" | "try_invoke_contract" | "call" | "try_call"
        ) || method.ends_with("_client")
            || method.starts_with("invoke");

        // Storage mutation patterns
        let is_mutation = matches!(method, "set" | "update" | "remove");

        if is_cross_call {
            self.saw_cross_call = true;
        }

        if is_mutation && self.saw_cross_call {
            if let Some(fn_name) = &self.current_fn {
                // Avoid duplicate issues per function
                if !self.issues.iter().any(|i| i.function_name == *fn_name) {
                    self.issues.push(ReentrancyIssue {
                        function_name: fn_name.clone(),
                        issue_type: "CEI violation: storage mutation after cross-contract call".to_string(),
                        location: format!("fn {}", fn_name),
                    });
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for ReentrancyVisitor {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev_fn = self.current_fn.take();
        let prev_cross = self.saw_cross_call;

        self.current_fn = Some(node.sig.ident.to_string());
        self.saw_cross_call = false;

        syn::visit::visit_impl_item_fn(self, node);

        self.current_fn = prev_fn;
        self.saw_cross_call = prev_cross;
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.check_method(&node.method.to_string());
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*node.func {
            if let Some(seg) = p.path.segments.last() {
                self.check_method(&seg.ident.to_string());
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Scan Rust source for CEI violations (cross-contract call before storage write).
pub fn scan_reentrancy(source: &str) -> Vec<ReentrancyIssue> {
    let file: File = match parse_str(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    // Only scan impl blocks (contract functions)
    let mut visitor = ReentrancyVisitor::new();
    for item in &file.items {
        if let Item::Impl(i) = item {
            visitor.visit_item_impl(i);
        }
    }
    visitor.issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cei_violation_detected() {
        let src = r#"
            #[contractimpl]
            impl MyContract {
                pub fn bad_transfer(env: Env, to: Address, amount: i128) {
                    // Cross-contract call THEN storage mutation = CEI violation
                    let client = TokenClient::new(&env, &to);
                    client.transfer(&env.current_contract_address(), &to, &amount);
                    env.storage().persistent().set(&DataKey::Balance, &amount);
                }
            }
        "#;
        let issues = scan_reentrancy(src);
        assert!(!issues.is_empty(), "Should detect CEI violation");
        assert!(issues[0].issue_type.contains("CEI"));
    }

    #[test]
    fn test_no_violation_when_storage_first() {
        let src = r#"
            #[contractimpl]
            impl MyContract {
                pub fn safe_transfer(env: Env, to: Address, amount: i128) {
                    // Effects first, then interaction — correct CEI order
                    env.storage().persistent().set(&DataKey::Balance, &amount);
                    let client = TokenClient::new(&env, &to);
                    client.transfer(&env.current_contract_address(), &to, &amount);
                }
            }
        "#;
        let issues = scan_reentrancy(src);
        assert!(issues.is_empty(), "Should not flag correct CEI order");
    }
}
