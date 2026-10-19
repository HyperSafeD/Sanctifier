//! Z010 — Verifying-key rotation / upgrade without access control.
//!
//! The verifying key is the root of trust for every proof a ZK contract
//! accepts.  A public function that writes a new verifying key (or
//! trusted-setup parameter) to storage must first authenticate a privileged
//! admin (`require_auth` / `require_auth_for_args`); otherwise any caller can
//! swap in a key whose trapdoor they know and forge arbitrary proofs.  This is
//! the ZK analogue of S010's unauthenticated upgrade, applied to verifying-key
//! storage rather than contract WASM.
//!
//! The rule reuses the S010 upgrade-admin detection approach — a state write
//! with no preceding auth guard is a finding — but focuses on verifying-key
//! storage writes.
//!
//! See `docs/rules/Z010.md`.

use crate::rules::{Rule, RuleViolation, Severity};
use syn::visit::Visit;
use syn::{parse_str, ExprMethodCall, File, Item};

/// Lowercased fragments identifying a verifying key or trusted-setup value.
const VK_FRAGMENTS: &[&str] = &[
    "verifying_key",
    "verification_key",
    "verifyingkey",
    "verificationkey",
    "verifying",
    "verify_key",
    "vkey",
    "vk",
];

/// Method names that authenticate a privileged caller.
const AUTH_CALLS: &[&str] = &["require_auth", "require_auth_for_args"];

/// Z010 — verifying-key storage write without a preceding admin auth check.
pub struct ZkMissingVkRotationAccessControlRule;

impl ZkMissingVkRotationAccessControlRule {
    /// Create the rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkMissingVkRotationAccessControlRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Does this text mention a verifying key?
fn mentions_vk(text: &str) -> bool {
    VK_FRAGMENTS.iter().any(|fragment| text.contains(fragment))
}

/// Lowercased token soup for an arbitrary syn node.
fn tokens_lower<T: quote::ToTokens>(node: &T) -> String {
    quote::quote!(#node).to_string().to_lowercase()
}

/// In-order scan of a function body.  `auth_seen` flips the moment an auth call
/// is visited, so a VK write recorded while `auth_seen` is false is
/// unauthenticated (the auth guard did not precede it).
#[derive(Default)]
struct VkRotationScan {
    /// A VK storage write was seen before any auth call.
    vulnerable: bool,
    /// An `require_auth` / `require_auth_for_args` call has been visited.
    auth_seen: bool,
}

impl<'ast> Visit<'ast> for VkRotationScan {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        let method_lower = method.to_lowercase();

        if AUTH_CALLS.iter().any(|call| method_lower == *call) {
            self.auth_seen = true;
        }

        let receiver = tokens_lower(&node.receiver);
        if receiver.contains("storage") && (method_lower == "set" || method_lower == "update") {
            let args: Vec<String> = node.args.iter().map(tokens_lower).collect();
            if args.iter().any(|arg| mentions_vk(arg)) && !self.auth_seen {
                self.vulnerable = true;
            }
        }

        syn::visit::visit_expr_method_call(self, node);
    }
}

impl Rule for ZkMissingVkRotationAccessControlRule {
    fn name(&self) -> &str {
        "unprotected_vk_rotation"
    }

    fn description(&self) -> &str {
        "Detects public functions that write a verifying key to storage without a preceding require_auth/admin check"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            let Item::Impl(impl_block) = item else {
                continue;
            };
            for impl_item in &impl_block.items {
                let syn::ImplItem::Fn(f) = impl_item else {
                    continue;
                };
                if !matches!(f.vis, syn::Visibility::Public(_)) {
                    continue;
                }

                let mut scan = VkRotationScan::default();
                scan.visit_block(&f.block);
                if !scan.vulnerable {
                    continue;
                }

                let fn_name = f.sig.ident.to_string();
                let fn_line = f.sig.ident.span().start().line;
                violations.push(
                    RuleViolation::new(
                        self.name(),
                        Severity::Critical,
                        format!(
                            "Function '{}' writes a verifying key to storage without a preceding \
                             require_auth/admin check. Any caller can rotate the trusted verification \
                             parameters and forge proofs, making the contract's ZK guarantees meaningless.",
                            fn_name
                        ),
                        format!("{}:{}", fn_name, fn_line),
                    )
                    .with_suggestion(
                        "Authenticate a privileged admin before writing the verifying key, e.g. \
                         `admin.require_auth();` where `admin` is the stored admin address, or use \
                         `require_auth_for_args` to bind authorization to the rotation payload. \
                         See docs/rules/Z010.md."
                            .to_string(),
                    ),
                );
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_unauthenticated_vk_rotation() {
        let rule = ZkMissingVkRotationAccessControlRule::new();
        let source = r#"
            impl Verifier {
                pub fn rotate(env: Env, new_vk: Bytes) {
                    env.storage().instance().set(&symbol_short!("VK"), &new_vk);
                }
            }
        "#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "unauthenticated rotation must fire: {v:?}");
        assert_eq!(v[0].severity, Severity::Critical);
        assert!(v[0].message.contains("rotate"));
    }

    #[test]
    fn no_violation_when_admin_is_authenticated() {
        let rule = ZkMissingVkRotationAccessControlRule::new();
        let source = r#"
            impl Verifier {
                pub fn rotate(env: Env, admin: Address, new_vk: Bytes) {
                    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
                    if stored_admin != admin {
                        panic!("unauthorized");
                    }
                    admin.require_auth();
                    env.storage().instance().set(&symbol_short!("VK"), &new_vk);
                }
            }
        "#;
        assert!(
            rule.check(source).is_empty(),
            "authenticated rotation must not fire"
        );
    }

    #[test]
    fn does_not_flag_getter_or_non_vk_writes() {
        let rule = ZkMissingVkRotationAccessControlRule::new();
        let source = r#"
            impl Verifier {
                pub fn get_vk(env: Env) -> Bytes {
                    env.storage().instance().get(&symbol_short!("VK")).expect("not initialized")
                }
                pub fn set_fee(env: Env, fee: u32) {
                    env.storage().instance().set(&symbol_short!("FEE"), &fee);
                }
            }
        "#;
        assert!(rule.check(source).is_empty());
    }

    #[test]
    fn empty_and_unparseable_source_is_safe() {
        let rule = ZkMissingVkRotationAccessControlRule::new();
        assert!(rule.check("").is_empty());
        assert!(rule.check("not rust @@@").is_empty());
    }
}
