use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, Item};

/// Rule S030 — detects functions with multiple Address parameters that use
/// require_auth instead of require_auth_for_args, enabling replay/scope-confusion
/// attacks on multi-arg admin operations.
pub struct RequireAuthForArgsRule;

impl RequireAuthForArgsRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequireAuthForArgsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RequireAuthForArgsRule {
    fn name(&self) -> &str {
        "require_auth_for_args"
    }

    fn description(&self) -> &str {
        "Detects functions with multiple Address parameters that use require_auth instead of require_auth_for_args, enabling replay/scope-confusion attacks"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        // Only check public functions
                        if !matches!(method.vis, syn::Visibility::Public(_)) {
                            continue;
                        }

                        let fn_name = method.sig.ident.to_string();

                        // Skip reserved Soroban entrypoints
                        if is_reserved_soroban_entrypoint(&fn_name) {
                            continue;
                        }

                        // Count Address parameters (excluding Env)
                        let address_param_count = count_address_params(&method.sig);

                        // Only flag functions with 2+ Address parameters
                        if address_param_count < 2 {
                            continue;
                        }

                        let block = &method.block;
                        let body = quote::quote!(#block).to_string();

                        // Check if function has state mutations or external calls
                        if !has_sensitive_operations(&body) {
                            continue;
                        }

                        // Check if it uses require_auth but not require_auth_for_args
                        let has_require_auth =
                            body.contains("require_auth()") || body.contains("require_auth (");
                        let has_require_auth_for_args = body.contains("require_auth_for_args");

                        if has_require_auth && !has_require_auth_for_args {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Error,
                                    format!(
                                        "Function '{}' has {} Address parameters and uses require_auth() instead of require_auth_for_args() — vulnerable to replay/scope-confusion attacks",
                                        fn_name, address_param_count
                                    ),
                                    fn_name.clone(),
                                )
                                .with_suggestion(
                                    "Replace require_auth() with require_auth_for_args() to bind authorization to the exact call payload. \
                                     Example: address.require_auth_for_args((arg1, arg2, ...).into_val(&env)); \
                                     This prevents an attacker from replaying a signature with different argument combinations.".to_string()
                                ),
                            );
                        }
                    }
                }
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn is_reserved_soroban_entrypoint(fn_name: &str) -> bool {
    matches!(fn_name, "__constructor" | "__check_auth")
}

fn count_address_params(sig: &syn::Signature) -> usize {
    sig.inputs
        .iter()
        .filter(|arg| {
            if let syn::FnArg::Typed(pt) = arg {
                let ty = &pt.ty;
                let ty_str = quote::quote!(#ty).to_string();
                // Match Address type (with or without whitespace)
                ty_str.contains("Address") && !ty_str.contains("Env")
            } else {
                false
            }
        })
        .count()
}

fn has_sensitive_operations(body: &str) -> bool {
    // Check for storage mutations (handle both original and quote!-spaced forms)
    let has_storage_mutation = body.contains(".set(")
        || body.contains(". set (")
        || body.contains(".update(")
        || body.contains(". update (")
        || body.contains(".remove(")
        || body.contains(". remove (")
        || body.contains(".extend_ttl(")
        || body.contains(". extend_ttl (")
        || body.contains(".bump(")
        || body.contains(". bump (");

    // Check for external contract calls
    let has_external_call = body.contains("invoke_contract")
        || body.contains("try_invoke_contract")
        || body.contains(".call(")
        || body.contains(". call (")
        || body.contains(".try_call(")
        || body.contains(". try_call (");

    has_storage_mutation || has_external_call
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_multi_address_with_require_auth() {
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env, Address, symbol_short};

            #[contract]
            pub struct TestContract;

            #[contractimpl]
            impl TestContract {
                pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
                    caller.require_auth();
                    env.storage().instance().set(&symbol_short!("admin"), &new_admin);
                }
            }
        "#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("require_auth_for_args"));
    }

    #[test]
    fn test_allows_require_auth_for_args() {
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env, Address, symbol_short};

            #[contract]
            pub struct TestContract;

            #[contractimpl]
            impl TestContract {
                pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
                    caller.require_auth_for_args((new_admin.clone(),).into_val(&env));
                    env.storage().instance().set(&symbol_short!("admin"), &new_admin);
                }
            }
        "#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_ignores_single_address_param() {
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env, Address, symbol_short};

            #[contract]
            pub struct TestContract;

            #[contractimpl]
            impl TestContract {
                pub fn set_owner(env: Env, owner: Address) {
                    owner.require_auth();
                    env.storage().instance().set(&symbol_short!("owner"), &owner);
                }
            }
        "#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_ignores_read_only_functions() {
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env, Address};

            #[contract]
            pub struct TestContract;

            #[contractimpl]
            impl TestContract {
                pub fn check_permission(env: Env, user: Address, admin: Address) -> bool {
                    user.require_auth();
                    // No storage mutation, just reading
                    true
                }
            }
        "#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_flags_three_address_params() {
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env, Address, symbol_short};

            #[contract]
            pub struct TestContract;

            #[contractimpl]
            impl TestContract {
                pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                    spender.require_auth();
                    env.storage().instance().set(&symbol_short!("balance"), &amount);
                }
            }
        "#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("3 Address parameters"));
    }

    #[test]
    fn test_multi_function_impl_block() {
        // Same source used in CLI integration test — all 3 functions in one impl block
        let source = r#"
use soroban_sdk::{contract, contractimpl, Env, Address, symbol_short};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        env.storage().instance().set(&symbol_short!("admin"), &new_admin);
    }

    pub fn set_admin_safe(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth_for_args((new_admin.clone(),).into_val(&env));
        env.storage().instance().set(&symbol_short!("admin"), &new_admin);
    }

    pub fn set_owner(env: Env, owner: Address) {
        owner.require_auth();
        env.storage().instance().set(&symbol_short!("owner"), &owner);
    }
}
"#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        // Only set_admin should be flagged (2 Address params, require_auth not require_auth_for_args)
        assert_eq!(
            violations.len(),
            1,
            "Expected exactly 1 violation, got {:?}",
            violations
        );
        assert!(violations[0].message.contains("set_admin"));
    }

    #[test]
    fn test_ignores_private_functions() {
        let source = r#"
            use soroban_sdk::{contract, contractimpl, Env, Address, symbol_short};

            #[contract]
            pub struct TestContract;

            #[contractimpl]
            impl TestContract {
                fn internal_set_admin(env: Env, caller: Address, new_admin: Address) {
                    caller.require_auth();
                    env.storage().instance().set(&symbol_short!("admin"), &new_admin);
                }
            }
        "#;

        let rule = RequireAuthForArgsRule::new();
        let violations = rule.check(source);
        assert_eq!(violations.len(), 0);
    }
}
