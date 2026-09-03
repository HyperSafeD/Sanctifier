//! Fuzz-harness specification extraction — bridges static analysis to
//! dynamic analysis.
//!
//! This module walks the same `#[contractimpl]` blocks that
//! [`crate::contract_discovery`] discovers, but additionally extracts the
//! full, typed parameter list of every public function. The result — a
//! [`HarnessContract`] tree — is a source-agnostic description of a
//! contract's callable ABI that a code generator (see the CLI's `harness`
//! subcommand) can turn into native `afl.rs` / `honggfuzz` fuzz-target
//! scaffolds.
//!
//! # Why not reuse [`crate::contract_discovery::DiscoveredFunction`]?
//!
//! `DiscoveredFunction` intentionally only carries a function's `name` and
//! `line` — enough for call-graph and complexity analyses, but not enough to
//! generate a fuzz harness, which must know each parameter's Rust type in
//! order to derive an `Arbitrary`/`SorobanArbitrary` input value for it.
//!
//! # Usage
//!
//! ```rust,ignore
//! use sanctifier_core::{parser, harness_spec};
//!
//! let parsed = parser::parse_source(source)?;
//! for contract in harness_spec::extract_harness_contracts(&parsed.file) {
//!     for f in &contract.functions {
//!         println!("{}::{}", contract.struct_name, f.name);
//!         for p in &f.params {
//!             println!("  {}: {}", p.name, p.ty_tokens);
//!         }
//!     }
//! }
//! ```

use std::collections::BTreeMap;
use syn::{FnArg, ImplItem, Item, Pat, Type};

use crate::contract_discovery::{has_attr_named, type_to_name, RESERVED_ENTRYPOINTS};

// ── Public data types ─────────────────────────────────────────────────────────

/// A single typed parameter of a fuzzable contract function.
///
/// The mandatory leading `Env` parameter that every Soroban contract
/// function takes is deliberately excluded — a fuzz harness constructs its
/// own `Env` rather than deriving one from fuzzer input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessParam {
    /// Parameter name as written (falls back to `argN` for irrefutable
    /// patterns `syn` cannot name, e.g. destructuring).
    pub name: String,
    /// The parameter's type, rendered as valid Rust source (e.g. `"Address"`,
    /// `"BytesN<32>"`, `"i128"`, `"Vec<Address>"`).
    pub ty_tokens: String,
}

/// A public, non-reserved contract function together with its fuzzable
/// parameter list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessFunction {
    /// Function name (e.g. `"transfer"`).
    pub name: String,
    /// Typed parameters, in declaration order, excluding the leading `Env`.
    pub params: Vec<HarnessParam>,
}

/// A Soroban contract's fuzzable surface: every public, non-reserved
/// function reachable through a `#[contractimpl]` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessContract {
    /// Name of the contract struct.
    pub struct_name: String,
    /// Fuzzable functions, in source order.
    pub functions: Vec<HarnessFunction>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Extract the fuzzable ABI of every contract in a parsed source file.
///
/// Only functions inside `#[contractimpl]` blocks are considered, matching
/// [`crate::contract_discovery::discover_contracts`]'s notion of a
/// "contract". Reserved entry-points (`__constructor`, `__check_auth`) and
/// non-`pub` functions are excluded, since neither is meaningfully callable
/// through a generated client-based fuzz harness.
///
/// Returns one [`HarnessContract`] per unique struct name, in deterministic
/// (lexicographic) order. Contracts with zero fuzzable functions (e.g. only
/// reserved entry-points) are omitted entirely.
pub fn extract_harness_contracts(file: &syn::File) -> Vec<HarnessContract> {
    let mut by_name: BTreeMap<String, Vec<HarnessFunction>> = BTreeMap::new();

    for item in &file.items {
        let Item::Impl(impl_block) = item else {
            continue;
        };
        if !has_attr_named(&impl_block.attrs, "contractimpl") {
            continue;
        }

        let struct_name =
            type_to_name(&impl_block.self_ty).unwrap_or_else(|| "<unknown>".to_string());
        let entry = by_name.entry(struct_name).or_default();

        for impl_item in &impl_block.items {
            let ImplItem::Fn(f) = impl_item else {
                continue;
            };
            if !matches!(f.vis, syn::Visibility::Public(_)) {
                continue;
            }
            let name = f.sig.ident.to_string();
            if RESERVED_ENTRYPOINTS.contains(&name.as_str()) {
                continue;
            }

            let params = extract_params(f);
            entry.push(HarnessFunction { name, params });
        }
    }

    by_name
        .into_iter()
        .filter(|(_, functions)| !functions.is_empty())
        .map(|(struct_name, functions)| HarnessContract {
            struct_name,
            functions,
        })
        .collect()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Collects every non-`Env` parameter of a function signature as a
/// [`HarnessParam`].
///
/// By Soroban convention `Env` is always the first parameter of a public
/// contract function; any parameter whose rendered type is exactly `"Env"`
/// is skipped regardless of position, so unconventional signatures degrade
/// gracefully rather than producing a bogus fuzz field.
fn extract_params(f: &syn::ImplItemFn) -> Vec<HarnessParam> {
    let mut params = Vec::new();
    let mut anon_index = 0usize;

    for input in &f.sig.inputs {
        let FnArg::Typed(typed) = input else {
            continue; // `self` receivers do not occur in #[contractimpl] fns
        };

        let ty_tokens = render_type(&typed.ty);
        if ty_tokens == "Env" {
            continue;
        }

        let name = pattern_name(&typed.pat).unwrap_or_else(|| {
            let name = format!("arg{anon_index}");
            anon_index += 1;
            name
        });

        params.push(HarnessParam { name, ty_tokens });
    }

    params
}

/// Renders a `syn::Type` as compact, valid Rust source (e.g. `BytesN<32>`
/// rather than `syn`'s debug/token-stream spacing).
fn render_type(ty: &Type) -> String {
    match ty {
        Type::Group(group) => render_type(&group.elem),
        Type::Paren(paren) => render_type(&paren.elem),
        Type::Reference(reference) => format!("&{}", render_type(&reference.elem)),
        Type::Tuple(tuple) if tuple.elems.is_empty() => "()".to_string(),
        _ => normalize_tokens(&quote::quote!(#ty).to_string()),
    }
}

/// Collapses `quote!`'s token-by-token spacing (`"BytesN < 32 >"`) into
/// idiomatic Rust source (`"BytesN<32>"`).
fn normalize_tokens(input: &str) -> String {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let mut out = String::with_capacity(input.len());
    for (i, tok) in tokens.iter().enumerate() {
        if i > 0 {
            let prev = tokens[i - 1];
            let glued = matches!(prev, "<" | "(" | "&" | "::")
                || matches!(*tok, "<" | ">" | ")" | "," | "::");
            if !glued {
                out.push(' ');
            }
        }
        out.push_str(tok);
    }
    out
}

/// Best-effort extraction of a parameter's identifier from its pattern.
fn pattern_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(ident) => Some(ident.ident.to_string()),
        Pat::Reference(reference) => pattern_name(&reference.pat),
        Pat::Type(typed) => pattern_name(&typed.pat),
        Pat::Paren(paren) => pattern_name(&paren.pat),
        _ => None,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;

    fn file_from(src: &str) -> syn::File {
        parse_source(src).expect("test source must parse").file
    }

    #[test]
    fn empty_file_yields_no_contracts() {
        let file = file_from("   ");
        assert!(extract_harness_contracts(&file).is_empty());
    }

    #[test]
    fn contract_with_only_reserved_entrypoints_is_omitted() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl MyContract {
                pub fn __constructor(_env: Env) {}
            }
        "#,
        );
        assert!(extract_harness_contracts(&file).is_empty());
    }

    #[test]
    fn env_param_is_excluded_from_harness_params() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {}
            }
        "#,
        );
        let contracts = extract_harness_contracts(&file);
        assert_eq!(contracts.len(), 1);
        let f = &contracts[0].functions[0];
        assert_eq!(f.name, "transfer");
        assert_eq!(f.params.len(), 3, "Env must be excluded");
        assert_eq!(f.params[0].name, "from");
        assert_eq!(f.params[0].ty_tokens, "Address");
        assert_eq!(f.params[1].name, "to");
        assert_eq!(f.params[2].name, "amount");
        assert_eq!(f.params[2].ty_tokens, "i128");
    }

    #[test]
    fn generic_and_reference_types_render_idiomatically() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl Vault {
                pub fn deposit(env: Env, id: BytesN<32>, amounts: Vec<i128>, note: &Bytes) {}
            }
        "#,
        );
        let contracts = extract_harness_contracts(&file);
        let f = &contracts[0].functions[0];
        assert_eq!(f.params[0].ty_tokens, "BytesN<32>");
        assert_eq!(f.params[1].ty_tokens, "Vec<i128>");
        assert_eq!(f.params[2].ty_tokens, "&Bytes");
    }

    #[test]
    fn private_and_reserved_functions_are_excluded() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl Token {
                pub fn __constructor(_env: Env) {}
                fn internal(_env: Env) {}
                pub fn transfer(_env: Env, _amount: i128) {}
            }
        "#,
        );
        let contracts = extract_harness_contracts(&file);
        assert_eq!(contracts[0].functions.len(), 1);
        assert_eq!(contracts[0].functions[0].name, "transfer");
    }

    #[test]
    fn zero_argument_function_yields_empty_params() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl Counter {
                pub fn increment(_env: Env) -> u32 { 0 }
            }
        "#,
        );
        let contracts = extract_harness_contracts(&file);
        assert!(contracts[0].functions[0].params.is_empty());
    }

    #[test]
    fn multiple_contracts_are_each_extracted() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl TokenA {
                pub fn a(_env: Env, _x: i128) {}
            }
            #[contractimpl]
            impl TokenB {
                pub fn b(_env: Env, _y: u32) {}
            }
        "#,
        );
        let contracts = extract_harness_contracts(&file);
        assert_eq!(contracts.len(), 2);
        let names: Vec<&str> = contracts.iter().map(|c| c.struct_name.as_str()).collect();
        assert!(names.contains(&"TokenA"));
        assert!(names.contains(&"TokenB"));
    }

    #[test]
    fn unnamed_or_unpatternable_params_get_positional_fallback_names() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl Weird {
                pub fn f(_env: Env, (a, b): (i128, i128)) {}
            }
        "#,
        );
        let contracts = extract_harness_contracts(&file);
        let f = &contracts[0].functions[0];
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "arg0");
        assert_eq!(f.params[0].ty_tokens, "(i128, i128)");
    }
}
