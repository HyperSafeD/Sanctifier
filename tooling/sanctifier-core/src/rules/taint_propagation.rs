//! Rule S026 — taint propagation through tuple and struct destructures.
//!
//! Tracks user-controlled data (function parameters marked as tainted) through
//! variable assignments, including `let (a, b) = ...` (Pat::Tuple) and
//! `let Foo { x, y } = ...` (Pat::Struct) destructures.  Emits a finding when
//! a tainted value reaches a sensitive sink (storage write or external call)
//! without an intervening `require_auth` or explicit validation.
//!
//! The actual dataflow is delegated to [`crate::taint_engine`], which runs a
//! fixed-point analysis over an intra-procedural CFG ([`crate::cfg`]) rather
//! than a single top-to-bottom AST walk. That matters for control flow this
//! rule previously missed entirely: a `for`/`while` loop body that taints a
//! variable used by a later iteration, or a branch that only conditionally
//! introduces taint before a sink is reached after the branches join.

use super::{Rule, RuleViolation, Severity};
use crate::taint_engine;
use std::collections::HashSet;
use syn::{parse_str, File, Item};

pub struct TaintPropagationRule;

impl TaintPropagationRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaintPropagationRule {
    fn default() -> Self {
        Self::new()
    }
}

// ── Rule impl ──────────────────────────────────────────────────────────────────

impl Rule for TaintPropagationRule {
    fn name(&self) -> &str {
        "taint_propagation"
    }

    fn description(&self) -> &str {
        "Tracks user-controlled data through the function's control-flow graph and flags \
         when tainted values reach storage or external-call sinks without auth"
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
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if !matches!(f.vis, syn::Visibility::Public(_)) {
                            continue;
                        }

                        let fn_name = f.sig.ident.to_string();

                        // Seed taint from parameters (excluding `env: Env` and `self`)
                        let sources = collect_param_names(&f.sig);
                        if sources.is_empty() {
                            continue;
                        }

                        let findings = taint_engine::analyze(&f.block, sources);

                        for finding in findings {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "Function '{}': tainted variable '{}' reaches '{}' \
                                         sink without prior require_auth",
                                        fn_name, finding.var, finding.sink
                                    ),
                                    format!("{}:{}", fn_name, finding.line),
                                )
                                .with_suggestion(
                                    "Call require_auth() on any address parameter before using \
                                     user-controlled data in storage or external calls."
                                        .to_string(),
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

// ── Parameter extraction ───────────────────────────────────────────────────────

fn collect_param_names(sig: &syn::Signature) -> HashSet<String> {
    let mut names = HashSet::new();
    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pt) = arg {
            // Skip Env parameters
            let ty_str = quote::quote!(#pt.ty).to_string();
            if ty_str.contains("Env") {
                continue;
            }
            taint_engine::collect_pat_idents(&pt.pat, &mut names);
        }
    }
    names
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> TaintPropagationRule {
        TaintPropagationRule::new()
    }

    #[test]
    fn detects_taint_through_tuple_destructure() {
        // Taint flows: user_data → (a, b) via tuple destructure → storage set with a
        let source = r#"
            impl MyContract {
                pub fn store_pair(env: Env, user_data: (Symbol, i128)) {
                    let (key, val) = user_data;
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "taint through tuple destructure must be flagged"
        );
        assert!(v[0].message.contains("store_pair"));
    }

    #[test]
    fn detects_taint_through_struct_destructure() {
        let source = r#"
            impl MyContract {
                pub fn store_record(env: Env, record: MyRecord) {
                    let MyRecord { key, value } = record;
                    env.storage().persistent().set(&key, &value);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "taint through struct destructure must be flagged"
        );
    }

    #[test]
    fn no_violation_when_require_auth_present() {
        let source = r#"
            impl MyContract {
                pub fn store_pair(env: Env, caller: Address, user_data: (Symbol, i128)) {
                    caller.require_auth();
                    let (key, val) = user_data;
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            v.is_empty(),
            "function with require_auth must not be flagged"
        );
    }

    #[test]
    fn no_violation_for_private_function() {
        let source = r#"
            impl MyContract {
                fn internal_store(env: Env, key: Symbol, val: i128) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            v.is_empty(),
            "private functions are not entry points and must not be flagged"
        );
    }

    #[test]
    fn direct_param_taint_to_storage() {
        // No destructure — param goes directly to storage key
        let source = r#"
            impl MyContract {
                pub fn bad_set(env: Env, key: Symbol, val: i128) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(!v.is_empty(), "direct taint to storage must be flagged");
    }

    #[test]
    fn empty_source_no_panic() {
        assert!(rule().check("").is_empty());
    }

    #[test]
    fn detects_taint_introduced_inside_for_loop_body() {
        // Regression test for the bug #401 exists to fix: a pure top-to-bottom
        // AST walk never visited for-loop bodies at all, so taint flowing
        // from a tainted Vec parameter into the per-iteration loop variable
        // was invisible. The CFG-backed taint engine must catch it.
        let source = r#"
            impl Batch {
                pub fn process(env: Env, recipients: Vec<Symbol>) {
                    let mut key = default_key;
                    for r in recipients.iter() {
                        key = r;
                        env.storage().persistent().set(&key, &1);
                    }
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "taint introduced inside a for-loop body must be flagged"
        );
    }

    #[test]
    fn detects_taint_from_one_if_branch_after_join() {
        let source = r#"
            impl MyContract {
                pub fn store(env: Env, user_input: Symbol) {
                    let mut key = default_key;
                    if some_cond {
                        key = user_input;
                    }
                    env.storage().persistent().set(&key, &1);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "taint introduced in only one if-branch must still be flagged after the join"
        );
    }
}
