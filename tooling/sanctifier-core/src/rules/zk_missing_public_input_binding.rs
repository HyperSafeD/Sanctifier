//! Z003 — Missing public-input binding (proof malleability).
//!
//! A ZK proof only attests to the statement encoded in its *public inputs*. If a
//! verifier call site passes a public-input vector that omits a security-relevant
//! runtime value — the recipient address, the transfer amount, the caller — then a
//! proof that was valid for one transaction is equally valid for another. An
//! attacker replays the proof with an attacker-favourable value for the unbound
//! parameter and redirects the effect.
//!
//! Detection is a heuristic, contract-side dataflow check. For each function that
//! calls a verifier we:
//!
//! 1. collect the expressions passed to the verifier as public inputs,
//! 2. transitively expand any local `let` bindings those expressions reference,
//!    so `let h = hash_address(&env, &recipient); verify(.., &vec![h])` counts as
//!    binding `recipient`,
//! 3. collect security-relevant function parameters that are still *used* after
//!    the verifier call, and
//! 4. flag the ones that never made it into the public inputs.
//!
//! See `docs/rules/Z003.md`.

use std::collections::{BTreeSet, HashMap};

use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, punctuated::Punctuated, token::Comma, Expr, File, FnArg, Item, Pat, Stmt};

/// Function names that perform on-chain proof verification.
const VERIFIER_FNS: &[&str] = &[
    "verify_proof",
    "groth16_verify",
    "verify_groth16",
    "snark_verify",
    "verify_snark",
    "plonk_verify",
    "verify_plonk",
    "verify_zk_proof",
];

/// Parameter-name fragments that denote a value an attacker would want to swap.
/// Matching is done on the lowercased parameter name.
const SECURITY_RELEVANT: &[&str] = &[
    "recipient",
    "receiver",
    "beneficiary",
    "amount",
    "value",
    "caller",
    "destination",
    "to_account",
    "payee",
];

/// Parameter names that are plumbing, never a binding target.
const IGNORED_PARAMS: &[&str] = &["env", "e", "proof", "public_inputs", "pub_inputs", "inputs"];

/// Z003 — verifier call whose public inputs omit a security-relevant value.
pub struct ZkMissingPublicInputBindingRule;

impl ZkMissingPublicInputBindingRule {
    /// Create the rule.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkMissingPublicInputBindingRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Lowercase token soup for an arbitrary syn node.
fn tokens_of<T: quote::ToTokens>(node: &T) -> String {
    quote::quote!(#node).to_string()
}

/// Split a token string into bare identifiers.
fn identifiers(tokens: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut current = String::new();
    for ch in tokens.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else if !current.is_empty() {
            out.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.insert(current);
    }
    out
}

/// True when this call expression targets one of the known verifier functions.
fn verifier_call_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Call(call) => {
            let path = tokens_of(&*call.func);
            VERIFIER_FNS
                .iter()
                .find(|name| path.contains(*name))
                .map(|name| (*name).to_string())
        }
        Expr::MethodCall(mc) => {
            let name = mc.method.to_string();
            VERIFIER_FNS
                .iter()
                .find(|candidate| name.contains(*candidate))
                .map(|candidate| (*candidate).to_string())
        }
        _ => None,
    }
}

/// Find the first verifier call anywhere inside an expression, returning its
/// argument list.
fn find_verifier_args(expr: &Expr) -> Option<(String, Punctuated<Expr, Comma>)> {
    if let Some(name) = verifier_call_name(expr) {
        let args = match expr {
            Expr::Call(call) => call.args.clone(),
            Expr::MethodCall(mc) => mc.args.clone(),
            _ => Punctuated::new(),
        };
        return Some((name, args));
    }

    // Recurse through the common wrappers: `verify(..).expect(..)`, `!verify(..)`,
    // `if verify(..) { .. }`, `let ok = verify(..);`.
    match expr {
        Expr::MethodCall(mc) => find_verifier_args(&mc.receiver),
        Expr::Try(t) => find_verifier_args(&t.expr),
        Expr::Unary(u) => find_verifier_args(&u.expr),
        Expr::Paren(p) => find_verifier_args(&p.expr),
        Expr::Reference(r) => find_verifier_args(&r.expr),
        Expr::Binary(b) => find_verifier_args(&b.left).or_else(|| find_verifier_args(&b.right)),
        Expr::Macro(m) => {
            // assert!(verify_proof(..)) — fall back to a token scan.
            let toks = tokens_of(m);
            if VERIFIER_FNS.iter().any(|f| toks.contains(f)) {
                Some((
                    VERIFIER_FNS
                        .iter()
                        .find(|f| toks.contains(*f))
                        .map(|f| (*f).to_string())
                        .unwrap_or_default(),
                    Punctuated::new(),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Collect `let name = <expr>;` bindings from a statement list, keyed by name.
fn local_bindings(stmts: &[Stmt]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for stmt in stmts {
        if let Stmt::Local(local) = stmt {
            let name = match &local.pat {
                Pat::Ident(id) => id.ident.to_string(),
                Pat::Type(pt) => match &*pt.pat {
                    Pat::Ident(id) => id.ident.to_string(),
                    _ => continue,
                },
                _ => continue,
            };
            if let Some(init) = &local.init {
                map.insert(name, tokens_of(&*init.expr));
            }
        }
    }
    map
}

/// Expand `seed` identifiers through local `let` bindings, up to a fixed depth,
/// so indirect binding (`let h = hash(&recipient)`) still counts.
fn expand_through_bindings(
    seed: BTreeSet<String>,
    bindings: &HashMap<String, String>,
) -> BTreeSet<String> {
    let mut resolved = seed;
    // Depth cap keeps this linear on pathological inputs; 8 hops is far beyond
    // anything a readable verifier entry point needs.
    for _ in 0..8 {
        let mut added = BTreeSet::new();
        for ident in &resolved {
            if let Some(init) = bindings.get(ident) {
                for nested in identifiers(init) {
                    if !resolved.contains(&nested) {
                        added.insert(nested);
                    }
                }
            }
        }
        if added.is_empty() {
            break;
        }
        resolved.extend(added);
    }
    resolved
}

/// Security-relevant parameters of a function signature, in declaration order.
fn security_relevant_params(sig: &syn::Signature) -> Vec<String> {
    let mut params = Vec::new();
    for arg in &sig.inputs {
        let FnArg::Typed(pt) = arg else { continue };
        let Pat::Ident(id) = &*pt.pat else { continue };
        let name = id.ident.to_string();
        let lower = name.to_lowercase();

        if IGNORED_PARAMS.contains(&lower.as_str()) {
            continue;
        }

        let ty = tokens_of(&*pt.ty);
        let name_matches = SECURITY_RELEVANT
            .iter()
            .any(|fragment| lower.contains(fragment));
        // An `Address` parameter is a binding target regardless of its name —
        // it is the classic redirect vector.
        let type_matches = ty.contains("Address");

        if name_matches || type_matches {
            params.push(name);
        }
    }
    params
}

/// Analyse one function body, returning the unbound parameter names (if any).
fn unbound_params(sig: &syn::Signature, stmts: &[Stmt]) -> Vec<String> {
    let candidates = security_relevant_params(sig);
    if candidates.is_empty() {
        return Vec::new();
    }

    // Locate the verifier call and capture its arguments.
    let mut verify_idx = None;
    let mut verifier_args: Punctuated<Expr, Comma> = Punctuated::new();
    for (idx, stmt) in stmts.iter().enumerate() {
        let expr = match stmt {
            Stmt::Expr(e, _) => Some(e.clone()),
            Stmt::Local(local) => local.init.as_ref().map(|i| (*i.expr).clone()),
            _ => None,
        };
        if let Some(expr) = expr {
            if let Some((_, args)) = find_verifier_args(&expr) {
                verify_idx = Some(idx);
                verifier_args = args;
                break;
            }
        }
    }

    let Some(verify_idx) = verify_idx else {
        return Vec::new();
    };

    let bindings = local_bindings(stmts);

    // Everything reachable from the verifier's arguments is "bound".
    let mut bound_seed = BTreeSet::new();
    for arg in &verifier_args {
        bound_seed.extend(identifiers(&tokens_of(arg)));
    }
    let bound = expand_through_bindings(bound_seed, &bindings);

    // A parameter only matters if the function still acts on it after the proof
    // was checked — that is where the redirect happens.
    let mut used_after = BTreeSet::new();
    for stmt in stmts.iter().skip(verify_idx + 1) {
        used_after.extend(identifiers(&tokens_of(stmt)));
    }

    candidates
        .into_iter()
        .filter(|param| used_after.contains(param) && !bound.contains(param))
        .collect()
}

impl Rule for ZkMissingPublicInputBindingRule {
    fn name(&self) -> &str {
        "missing_public_input_binding"
    }

    fn description(&self) -> &str {
        "Detects verifier calls whose public inputs omit a security-relevant value used after verification (proof malleability)"
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
                let unbound = unbound_params(&f.sig, &f.block.stmts);
                if unbound.is_empty() {
                    continue;
                }

                let fn_name = f.sig.ident.to_string();
                violations.push(
                    RuleViolation::new(
                        self.name(),
                        Severity::Critical,
                        format!(
                            "Function '{}' verifies a ZK proof but does not bind {} to the public \
                             inputs. A valid proof can be replayed with a different value for {}, \
                             redirecting the operation.",
                            fn_name,
                            quoted_list(&unbound),
                            if unbound.len() == 1 { "it" } else { "them" },
                        ),
                        fn_name,
                    )
                    .with_suggestion(format!(
                        "Include {} in the public-input vector passed to the verifier so the \
                         circuit commits to this transaction's context. Hash non-field values \
                         (e.g. an Address) into a field element first, and use the same encoding \
                         the circuit expects.",
                        quoted_list(&unbound)
                    )),
                );
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Render `["a", "b"]` as `'a', 'b'` for message text.
fn quoted_list(items: &[String]) -> String {
    items
        .iter()
        .map(|i| format!("'{i}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_recipient_used_after_verify_but_not_in_public_inputs() {
        let rule = ZkMissingPublicInputBindingRule::new();
        let source = r#"
            impl Shielded {
                pub fn withdraw(env: Env, proof: Vec<u64>, recipient: Address, amount: i128) {
                    let public_inputs = vec![&env, amount as u64];
                    verify_proof(&env, &proof, &public_inputs);
                    token_client.transfer(&env.current_contract_address(), &recipient, &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert_eq!(v.len(), 1, "unbound recipient must fire exactly once");
        assert!(v[0].message.contains("withdraw"));
        assert!(v[0].message.contains("recipient"));
    }

    #[test]
    fn no_violation_when_recipient_is_bound_indirectly() {
        let rule = ZkMissingPublicInputBindingRule::new();
        let source = r#"
            impl Shielded {
                pub fn withdraw(env: Env, proof: Vec<u64>, recipient: Address, amount: i128) {
                    let recipient_hash = hash_address(&env, &recipient);
                    let public_inputs = vec![&env, recipient_hash, amount as u64];
                    verify_proof(&env, &proof, &public_inputs);
                    token_client.transfer(&env.current_contract_address(), &recipient, &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(
            v.is_empty(),
            "indirect binding via a hash must not fire: {v:?}"
        );
    }

    #[test]
    fn no_violation_when_value_passed_directly_to_verifier() {
        let rule = ZkMissingPublicInputBindingRule::new();
        let source = r#"
            impl Shielded {
                pub fn claim(env: Env, proof: Vec<u64>, recipient: Address, amount: i128) {
                    groth16_verify(&env, &proof, &recipient, &amount);
                    do_transfer(&env, &recipient, &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "direct binding must not fire: {v:?}");
    }

    #[test]
    fn no_violation_when_value_is_unused_after_verification() {
        let rule = ZkMissingPublicInputBindingRule::new();
        let source = r#"
            impl Shielded {
                pub fn attest(env: Env, proof: Vec<u64>, recipient: Address) {
                    let public_inputs = vec![&env, 1u64];
                    verify_proof(&env, &proof, &public_inputs);
                    env.events().publish((symbol_short!("OK"),), 1u32);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(
            v.is_empty(),
            "a value never used after verification cannot be redirected: {v:?}"
        );
    }

    #[test]
    fn no_violation_without_a_verifier_call() {
        let rule = ZkMissingPublicInputBindingRule::new();
        let source = r#"
            impl Plain {
                pub fn transfer(env: Env, recipient: Address, amount: i128) {
                    token_client.transfer(&env.current_contract_address(), &recipient, &amount);
                }
            }
        "#;
        assert!(rule.check(source).is_empty());
    }

    #[test]
    fn empty_and_unparseable_source_is_safe() {
        let rule = ZkMissingPublicInputBindingRule::new();
        assert!(rule.check("").is_empty());
        assert!(rule.check("this is not rust {{{").is_empty());
    }
}
