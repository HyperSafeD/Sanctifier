//! Detects large/per-user datasets stored in `Instance` storage instead of
//! `Persistent`, which causes ledger entry bloat and ballooning rent costs.
//!
//! In Soroban, `Instance` storage is a single ledger entry shared by the whole
//! contract.  Storing per-user data (balances, profiles, allowances) there
//! causes that single entry to grow without bound.  Such data must live in
//! `Persistent` (or `Temporary`) storage, where each key is its own ledger
//! entry with independent rent.

use crate::rules::{Rule, RuleViolation, Severity};
use syn::{
    parse_str,
    visit::{self, Visit},
    Expr, ExprMethodCall, File,
};

// ── Heuristic keyword sets ────────────────────────────────────────────────────

/// Key-name fragments that strongly suggest per-user / per-account data.
/// Matched case-insensitively against the stringified key argument.
const PER_USER_HINTS: &[&str] = &[
    "balance",
    "allowance",
    "profile",
    "account",
    "user",
    "owner",
    "holder",
    "stake",
    "deposit",
    "vote",
    "record",
    "entry",
    "position",
    "nonce",
    "reward",
    "claim",
];

/// Storage operations that write data (reads are lower risk but still flagged
/// because the data must have been written somewhere).
const WRITE_OPS: &[&str] = &["set", "update", "try_update"];

// ── Visitor ───────────────────────────────────────────────────────────────────

struct InstanceMisuseVisitor {
    violations: Vec<InstanceMisuseViolation>,
    current_fn: String,
}

struct InstanceMisuseViolation {
    fn_name: String,
    key_hint: String,
    line: usize,
}

impl InstanceMisuseVisitor {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            current_fn: String::new(),
        }
    }

    /// Returns `true` when the expression looks like a per-user key.
    fn is_per_user_key(expr: &Expr) -> bool {
        let text = quote::quote!(#expr).to_string().to_lowercase();
        PER_USER_HINTS.iter().any(|hint| text.contains(hint))
    }

    /// Extract a short human-readable label from the key expression.
    fn key_label(expr: &Expr) -> String {
        let raw = quote::quote!(#expr).to_string();
        // Trim to a reasonable length for the message.
        if raw.len() > 60 {
            format!("{}…", &raw[..60])
        } else {
            raw
        }
    }
}

impl<'ast> Visit<'ast> for InstanceMisuseVisitor {
    // Track which function we are currently inside.
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev = std::mem::replace(&mut self.current_fn, node.sig.ident.to_string());
        visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev = std::mem::replace(&mut self.current_fn, node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();

        // We are looking for the pattern:
        //   env.storage().instance().<write_op>(key, …)
        // The receiver chain is:  env → storage() → instance()
        if WRITE_OPS.contains(&method.as_str()) {
            if let Some(key_arg) = node.args.first() {
                if Self::is_per_user_key(key_arg) && is_instance_chain(&node.receiver) {
                    let line = line_of(&node.receiver);
                    self.violations.push(InstanceMisuseViolation {
                        fn_name: self.current_fn.clone(),
                        key_hint: Self::key_label(key_arg),
                        line,
                    });
                }
            }
        }

        // Continue walking.
        visit::visit_expr_method_call(self, node);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns `true` when `expr` ends with `.instance()` (i.e. the receiver of
/// the storage operation is the instance store).
fn is_instance_chain(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(mc) => {
            if mc.method == "instance" {
                return true;
            }
            is_instance_chain(&mc.receiver)
        }
        _ => false,
    }
}

/// Best-effort line number from a span (falls back to 0).
fn line_of(expr: &Expr) -> usize {
    use syn::spanned::Spanned;
    expr.span().start().line
}

// ── Rule ──────────────────────────────────────────────────────────────────────

/// Rule that flags per-user / large-dataset writes to `Instance` storage.
pub struct InstanceStorageMisuseRule;

impl InstanceStorageMisuseRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for InstanceStorageMisuseRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for InstanceStorageMisuseRule {
    fn name(&self) -> &str {
        "instance_storage_misuse"
    }

    fn description(&self) -> &str {
        "Flags per-user or large datasets (balances, profiles, allowances) stored \
         in Instance storage instead of Persistent, which causes a single ledger \
         entry to balloon and drives up rent costs for all users"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = InstanceMisuseVisitor::new();
        visitor.visit_file(&file);

        visitor
            .violations
            .into_iter()
            .map(|v| {
                RuleViolation::new(
                    self.name(),
                    Severity::Warning,
                    format!(
                        "Function '{}' stores per-user data key `{}` in Instance storage (line {}). \
                         Instance storage is a single shared ledger entry; per-user data will cause \
                         it to grow unboundedly, ballooning rent costs.",
                        v.fn_name, v.key_hint, v.line
                    ),
                    format!("{}:{}", v.fn_name, v.line),
                )
                .with_suggestion(
                    "Move per-user / per-account data to `env.storage().persistent()` \
                     (or `temporary()` for short-lived state). Reserve `instance()` for \
                     contract-wide singletons such as admin address, decimals, or feature flags."
                        .to_string(),
                )
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_balance_written_to_instance_storage() {
        let rule = InstanceStorageMisuseRule::new();
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn mint(env: Env, to: Address, amount: i128) {
                    env.storage().instance().set(&DataKey::Balance(to), &amount);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "balance in instance storage must be flagged"
        );
        assert!(violations[0].message.contains("mint"));
    }

    #[test]
    fn flags_user_profile_written_to_instance_storage() {
        let rule = InstanceStorageMisuseRule::new();
        let source = r#"
            #[contractimpl]
            impl Registry {
                pub fn register(env: Env, user: Address, profile: UserProfile) {
                    env.storage().instance().set(&DataKey::Profile(user), &profile);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "user profile in instance storage must be flagged"
        );
    }

    #[test]
    fn no_violation_for_singleton_in_instance_storage() {
        let rule = InstanceStorageMisuseRule::new();
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    env.storage().instance().set(&DataKey::Admin, &admin);
                    env.storage().instance().set(&DataKey::Decimals, &7u32);
                }
            }
        "#;
        // Admin and Decimals are singletons — should not be flagged.
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "singleton keys must not be flagged: {:?}",
            violations
        );
    }

    #[test]
    fn no_violation_for_balance_in_persistent_storage() {
        let rule = InstanceStorageMisuseRule::new();
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn mint(env: Env, to: Address, amount: i128) {
                    env.storage().persistent().set(&DataKey::Balance(to), &amount);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "persistent storage is correct — must not be flagged"
        );
    }

    #[test]
    fn flags_allowance_written_to_instance_storage() {
        let rule = InstanceStorageMisuseRule::new();
        let source = r#"
            #[contractimpl]
            impl Token {
                pub fn approve(env: Env, from: Address, spender: Address, amount: i128) {
                    env.storage().instance().set(&DataKey::Allowance(from, spender), &amount);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "allowance in instance storage must be flagged"
        );
    }

    #[test]
    fn empty_source_produces_no_violations() {
        let rule = InstanceStorageMisuseRule::new();
        assert!(rule.check("").is_empty());
    }

    #[test]
    fn invalid_source_produces_no_panic() {
        let rule = InstanceStorageMisuseRule::new();
        assert!(rule.check("not valid rust }{{{").is_empty());
    }
}
