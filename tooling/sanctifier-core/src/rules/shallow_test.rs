//! S023 — `#[test]` functions that never reference a `ContractClient`.
//!
//! Unit tests that call internal helpers directly instead of going through the
//! generated `ContractClient` miss real serialization and auth paths that only
//! activate when the host-function boundary is exercised.  Prefer
//! `Env::register_contract` + the generated client pattern so that the full
//! Soroban execution stack is covered.

use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, Item};

/// Rule that flags `#[test]` functions in a contract crate that never reference
/// a `ContractClient`-style type.
pub struct ShallowTestRule;

impl ShallowTestRule {
    /// Create a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShallowTestRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ShallowTestRule {
    fn name(&self) -> &str {
        "shallow_test"
    }

    fn description(&self) -> &str {
        "Flags #[test] functions that never reference a ContractClient, \
         meaning they bypass the host-function boundary and miss serialization / auth paths"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        // Only analyse files that contain a `#[contractimpl]` block — plain
        // library or helper files are out of scope.
        if !source_has_contractimpl(&file) {
            return vec![];
        }

        let mut violations = Vec::new();
        collect_violations_from_items(&file.items, &mut violations);
        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn source_has_contractimpl(file: &File) -> bool {
    for item in &file.items {
        match item {
            Item::Impl(i) => {
                if has_contractimpl_attr(&i.attrs) {
                    return true;
                }
            }
            Item::Mod(m) => {
                if let Some((_, items)) = &m.content {
                    for inner in items {
                        if let Item::Impl(i) = inner {
                            if has_contractimpl_attr(&i.attrs) {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn has_contractimpl_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("contractimpl"))
}

fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("test"))
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && quote::quote!(#a).to_string().contains("test"))
}

/// Returns `true` when the token stream of `block` contains a reference to a
/// type or variable whose name ends with `Client` (the Soroban SDK convention
/// for generated contract clients).
fn block_references_client(block: &syn::Block) -> bool {
    let tokens = quote::quote!(#block).to_string();
    references_client_in_str(&tokens)
}

fn references_client_in_str(s: &str) -> bool {
    // Matches identifiers ending with `Client` — the generated client pattern
    // (`MyContractClient`, `TokenClient`, …) and `Env::register_contract`.
    s.contains("Client") || s.contains("register_contract")
}

fn collect_violations_from_items(items: &[Item], violations: &mut Vec<RuleViolation>) {
    for item in items {
        match item {
            // Top-level `#[test]` free functions
            Item::Fn(f) if has_test_attr(&f.attrs) => {
                if !block_references_client(&f.block) {
                    violations.push(make_violation(f.sig.ident.to_string()));
                }
            }
            // `#[cfg(test)]` modules — descend into them
            Item::Mod(m) if is_cfg_test(&m.attrs) => {
                if let Some((_, inner_items)) = &m.content {
                    collect_violations_from_items(inner_items, violations);
                }
            }
            // `mod tests { … }` even without cfg attr
            Item::Mod(m) => {
                if let Some((_, inner_items)) = &m.content {
                    collect_violations_from_items(inner_items, violations);
                }
            }
            // Test functions inside `impl` blocks (e.g. integration helper impls)
            Item::Impl(i) if is_cfg_test(&i.attrs) => {
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if has_test_attr(&f.attrs) && !block_references_client(&f.block) {
                            violations.push(make_violation(f.sig.ident.to_string()));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn make_violation(fn_name: String) -> RuleViolation {
    RuleViolation::new(
        "shallow_test",
        Severity::Info,
        format!(
            "Test `{}` never references a ContractClient — it may bypass \
             serialization and auth paths exercised by the host-function boundary",
            fn_name
        ),
        fn_name,
    )
    .with_suggestion(
        "Use `Env::register_contract` to deploy the contract under test and \
         call it through the generated `<ContractName>Client` to exercise the \
         full Soroban execution stack"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_test_without_client_reference() {
        let rule = ShallowTestRule::new();
        let source = r#"
            use soroban_sdk::{Env, contract, contractimpl};

            #[contract]
            pub struct Counter;

            #[contractimpl]
            impl Counter {
                pub fn increment(env: Env) -> u32 { 1 }
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn test_increment_internal() {
                    let env = Env::default();
                    assert_eq!(1, 1);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty(), "shallow test must be flagged");
        assert_eq!(violations[0].severity, Severity::Info);
        assert!(violations[0].suggestion.is_some());
    }

    #[test]
    fn does_not_flag_test_with_client() {
        let rule = ShallowTestRule::new();
        let source = r#"
            use soroban_sdk::{Env, contract, contractimpl};

            #[contract]
            pub struct Counter;

            #[contractimpl]
            impl Counter {
                pub fn increment(env: Env) -> u32 { 1 }
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                #[test]
                fn test_via_client() {
                    let env = Env::default();
                    let id = env.register_contract(None, Counter);
                    let client = CounterClient::new(&env, &id);
                    assert_eq!(client.increment(), 1);
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty(), "test using ContractClient must not be flagged");
    }

    #[test]
    fn skips_files_without_contractimpl() {
        let rule = ShallowTestRule::new();
        let source = r#"
            pub fn helper() -> u32 { 42 }

            #[cfg(test)]
            mod tests {
                #[test]
                fn test_helper() {
                    assert_eq!(42, super::helper());
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty(), "non-contract files must be skipped");
    }

    #[test]
    fn suggestion_mentions_register_contract() {
        let rule = ShallowTestRule::new();
        let source = r#"
            #[contractimpl]
            impl MyContract {
                pub fn do_thing(env: Env) {}
            }

            #[cfg(test)]
            mod tests {
                #[test]
                fn bare_test() {
                    let _ = 1 + 1;
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty());
        let suggestion = violations[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("register_contract"),
            "suggestion must mention register_contract"
        );
    }

    #[test]
    fn empty_source_produces_no_violations() {
        let rule = ShallowTestRule::new();
        assert!(rule.check("").is_empty());
    }
}
