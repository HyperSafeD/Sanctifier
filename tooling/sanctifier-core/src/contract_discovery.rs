//! Contract discovery — identifies `#[contract]`, `#[contractimpl]`, and
//! `#[contracttype]` items in a parsed Soroban source file.
//!
//! The discovery layer answers three questions per source file:
//! 1. Which structs are Soroban contract entry points (`#[contract]`)?
//! 2. Which impl blocks expose public functions (`#[contractimpl]`)?
//! 3. Which types define on-chain storage layout (`#[contracttype]`)?
//!
//! These answers feed every downstream analysis pass so they do not have to
//! re-implement the same attribute scanning and struct→impl mapping.
//!
//! # Usage
//!
//! ```rust,ignore
//! use sanctifier_core::{parser, contract_discovery};
//!
//! let parsed = parser::parse_source(source)?;
//! let contracts = contract_discovery::discover_contracts(&parsed.file);
//!
//! for contract in &contracts {
//!     println!("contract: {}", contract.struct_name);
//!     for f in contract.public_functions() {
//!         println!("  pub fn {} (line {})", f.name, f.line);
//!     }
//! }
//! ```
//!
//! # Mapping rules
//!
//! - A `#[contract]` struct with a matching `#[contractimpl]` block produces
//!   one [`DiscoveredContract`] with both `has_contract_attr` and
//!   `has_contractimpl` set to `true`.
//! - A `#[contractimpl]` block whose self-type has **no** `#[contract]` struct
//!   in the same file produces an orphan entry with `has_contract_attr: false`.
//! - A `#[contract]` struct with **no** impl block produces an entry with
//!   `has_contractimpl: false` and an empty `public_functions` list.
//! - `#[contracttype]` types are attached to every [`DiscoveredContract`]
//!   in the file (storage types are file-scoped, not per-contract).

use std::collections::BTreeMap;
use syn::{File, ImplItem, Item, Meta, Type};

// ── Public data types ─────────────────────────────────────────────────────────

/// A public function discovered inside a `#[contractimpl]` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFunction {
    /// Function name (e.g. `"transfer"`).
    pub name: String,
    /// 1-based source line where the function is declared.
    pub line: usize,
    /// `true` if this function is a reserved Soroban entry-point
    /// (`__constructor`, `__check_auth`).
    pub is_reserved: bool,
}

/// A `#[contracttype]` enum or struct found at the file level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageType {
    /// Name of the type (e.g. `"DataKey"`).
    pub name: String,
    /// `"struct"` or `"enum"`.
    pub kind: &'static str,
}

/// A Soroban contract discovered in a source file.
///
/// See the [module-level documentation](self) for the mapping rules that
/// determine how structs and impl blocks are combined into this structure.
#[derive(Debug, Clone)]
pub struct DiscoveredContract {
    /// Name of the contract struct (taken from the `#[contract]` struct or the
    /// self-type of the `#[contractimpl]` block when no struct was found).
    pub struct_name: String,
    /// Whether a `#[contract]` attribute was found on a struct with this name.
    pub has_contract_attr: bool,
    /// Whether at least one `#[contractimpl]` block was found for this struct.
    pub has_contractimpl: bool,
    pub(crate) fns: Vec<DiscoveredFunction>,
    /// All `#[contracttype]` types found in the file.
    pub storage_types: Vec<StorageType>,
}

impl DiscoveredContract {
    /// Returns only the non-reserved public functions.
    pub fn public_functions(&self) -> impl Iterator<Item = &DiscoveredFunction> {
        self.fns.iter().filter(|f| !f.is_reserved)
    }

    /// Returns all public functions, including reserved entry-points
    /// (`__constructor`, `__check_auth`).
    pub fn all_public_functions(&self) -> &[DiscoveredFunction] {
        &self.fns
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Soroban-defined reserved function names that are not user-callable.
pub(crate) const RESERVED_ENTRYPOINTS: &[&str] = &["__constructor", "__check_auth"];

pub(crate) fn has_attr_named(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        if let Meta::Path(path) = &attr.meta {
            path.is_ident(name) || path.segments.iter().any(|s| s.ident == name)
        } else {
            false
        }
    })
}

pub(crate) fn type_to_name(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        tp.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Discover all contracts and storage types in a parsed Soroban source file.
///
/// Returns one [`DiscoveredContract`] per unique struct name encountered via
/// either a `#[contract]` annotation or a `#[contractimpl]` impl block.
/// Results are returned in deterministic (lexicographic) order by struct name.
///
/// Returns an empty `Vec` when the file contains no Soroban-specific
/// attributes (i.e. plain Rust code or an empty AST).
pub fn discover_contracts(file: &File) -> Vec<DiscoveredContract> {
    // ── Phase 1: collect file-scoped #[contracttype] types ────────────────────
    let storage_types: Vec<StorageType> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(s) if has_attr_named(&s.attrs, "contracttype") => Some(StorageType {
                name: s.ident.to_string(),
                kind: "struct",
            }),
            Item::Enum(e) if has_attr_named(&e.attrs, "contracttype") => Some(StorageType {
                name: e.ident.to_string(),
                kind: "enum",
            }),
            _ => None,
        })
        .collect();

    // ── Phase 2: seed the map from #[contract] structs ────────────────────────
    let contract_struct_names: Vec<String> = file
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Struct(s) = item {
                if has_attr_named(&s.attrs, "contract") {
                    return Some(s.ident.to_string());
                }
            }
            None
        })
        .collect();

    let mut by_name: BTreeMap<String, DiscoveredContract> = contract_struct_names
        .iter()
        .map(|name| {
            (
                name.clone(),
                DiscoveredContract {
                    struct_name: name.clone(),
                    has_contract_attr: true,
                    has_contractimpl: false,
                    fns: vec![],
                    storage_types: storage_types.clone(),
                },
            )
        })
        .collect();

    // ── Phase 3: attach #[contractimpl] functions ─────────────────────────────
    for item in &file.items {
        let Item::Impl(impl_block) = item else {
            continue;
        };
        if !has_attr_named(&impl_block.attrs, "contractimpl") {
            continue;
        }

        let struct_name =
            type_to_name(&impl_block.self_ty).unwrap_or_else(|| "<unknown>".to_string());

        let entry = by_name
            .entry(struct_name.clone())
            .or_insert_with(|| DiscoveredContract {
                struct_name: struct_name.clone(),
                has_contract_attr: contract_struct_names.contains(&struct_name),
                has_contractimpl: false,
                fns: vec![],
                storage_types: storage_types.clone(),
            });
        entry.has_contractimpl = true;

        for impl_item in &impl_block.items {
            if let ImplItem::Fn(f) = impl_item {
                if matches!(f.vis, syn::Visibility::Public(_)) {
                    let name = f.sig.ident.to_string();
                    let line = f.sig.ident.span().start().line;
                    entry.fns.push(DiscoveredFunction {
                        is_reserved: RESERVED_ENTRYPOINTS.contains(&name.as_str()),
                        name,
                        line,
                    });
                }
            }
        }
    }

    by_name.into_values().collect()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;

    fn file_from(src: &str) -> syn::File {
        parse_source(src).expect("test source must parse").file
    }

    // ── No Soroban attributes ─────────────────────────────────────────────────

    #[test]
    fn empty_file_yields_no_contracts() {
        let file = file_from("   ");
        assert!(discover_contracts(&file).is_empty());
    }

    #[test]
    fn plain_impl_without_contractimpl_yields_no_contracts() {
        let file = file_from(
            r#"
            impl Foo {
                pub fn bar(_env: Env) {}
            }
        "#,
        );
        assert!(discover_contracts(&file).is_empty());
    }

    #[test]
    fn only_contracttype_without_contract_yields_no_contracts() {
        let file = file_from(
            r#"
            #[contracttype]
            pub enum DataKey { A, B }
        "#,
        );
        assert!(discover_contracts(&file).is_empty());
    }

    // ── #[contract] struct only ───────────────────────────────────────────────

    #[test]
    fn contract_struct_without_impl_is_discovered() {
        let file = file_from(
            r#"
            #[contract]
            pub struct MyContract;
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].struct_name, "MyContract");
        assert!(contracts[0].has_contract_attr);
        assert!(!contracts[0].has_contractimpl);
        assert!(contracts[0].fns.is_empty());
    }

    // ── #[contractimpl] only (orphan) ─────────────────────────────────────────

    #[test]
    fn contractimpl_without_contract_attr_is_discovered_as_orphan() {
        let file = file_from(
            r#"
            #[contractimpl]
            impl Orphan {
                pub fn hello(_env: Env) -> u32 { 42 }
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts.len(), 1);
        let c = &contracts[0];
        assert_eq!(c.struct_name, "Orphan");
        assert!(!c.has_contract_attr, "no #[contract] struct in source");
        assert!(c.has_contractimpl);
        assert_eq!(c.fns.len(), 1);
        assert_eq!(c.fns[0].name, "hello");
    }

    // ── Full contract (struct + impl) ─────────────────────────────────────────

    #[test]
    fn full_contract_collects_public_functions() {
        let file = file_from(
            r#"
            #[contract]
            pub struct Token;
            #[contractimpl]
            impl Token {
                pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}
                fn internal_helper() {}
                pub fn balance(_env: Env, _id: Address) -> i128 { 0 }
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts.len(), 1);
        let c = &contracts[0];
        assert!(c.has_contract_attr);
        assert!(c.has_contractimpl);
        assert_eq!(c.fns.len(), 2, "only public functions are collected");
        let names: Vec<&str> = c.fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"transfer"));
        assert!(names.contains(&"balance"));
        assert!(!names.contains(&"internal_helper"));
    }

    #[test]
    fn private_functions_are_excluded() {
        let file = file_from(
            r#"
            #[contract]
            pub struct Vault;
            #[contractimpl]
            impl Vault {
                pub fn deposit(_env: Env) {}
                fn _internal() {}
                pub(crate) fn semi_private() {}
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts[0].fns.len(), 1);
        assert_eq!(contracts[0].fns[0].name, "deposit");
    }

    // ── Reserved entry-points ─────────────────────────────────────────────────

    #[test]
    fn reserved_entrypoints_are_flagged_but_still_collected() {
        let file = file_from(
            r#"
            #[contract]
            pub struct MyContract;
            #[contractimpl]
            impl MyContract {
                pub fn __constructor(_env: Env) {}
                pub fn __check_auth(_env: Env) {}
                pub fn work(_env: Env) {}
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        let fns = contracts[0].all_public_functions();
        assert_eq!(fns.len(), 3);

        let constructor = fns.iter().find(|f| f.name == "__constructor").unwrap();
        assert!(constructor.is_reserved);

        let check_auth = fns.iter().find(|f| f.name == "__check_auth").unwrap();
        assert!(check_auth.is_reserved);

        let work = fns.iter().find(|f| f.name == "work").unwrap();
        assert!(!work.is_reserved);
    }

    #[test]
    fn public_functions_iterator_excludes_reserved() {
        let file = file_from(
            r#"
            #[contract]
            pub struct MyContract;
            #[contractimpl]
            impl MyContract {
                pub fn __constructor(_env: Env) {}
                pub fn do_work(_env: Env) {}
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        let names: Vec<&str> = contracts[0]
            .public_functions()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(names, vec!["do_work"]);
    }

    // ── Storage types ─────────────────────────────────────────────────────────

    #[test]
    fn contracttype_enum_is_collected_as_storage_type() {
        let file = file_from(
            r#"
            #[contracttype]
            pub enum DataKey { Admin, Balance }
            #[contract]
            pub struct Token;
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts[0].storage_types.len(), 1);
        assert_eq!(contracts[0].storage_types[0].name, "DataKey");
        assert_eq!(contracts[0].storage_types[0].kind, "enum");
    }

    #[test]
    fn contracttype_struct_is_collected_as_storage_type() {
        let file = file_from(
            r#"
            #[contracttype]
            pub struct Config { pub limit: u32 }
            #[contract]
            pub struct Token;
        "#,
        );
        let contracts = discover_contracts(&file);
        let st = &contracts[0].storage_types[0];
        assert_eq!(st.name, "Config");
        assert_eq!(st.kind, "struct");
    }

    #[test]
    fn multiple_contracttype_items_are_all_collected() {
        let file = file_from(
            r#"
            #[contracttype]
            pub enum DataKey { Admin, Balance }
            #[contracttype]
            pub struct Config { pub limit: u32 }
            #[contract]
            pub struct Token;
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts[0].storage_types.len(), 2);
        let names: Vec<&str> = contracts[0]
            .storage_types
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert!(names.contains(&"DataKey"));
        assert!(names.contains(&"Config"));
    }

    #[test]
    fn storage_types_are_shared_across_all_contracts() {
        let file = file_from(
            r#"
            #[contracttype]
            pub enum SharedKey { X }
            #[contract]
            pub struct A;
            #[contractimpl]
            impl A { pub fn a(_env: Env) {} }
            #[contract]
            pub struct B;
            #[contractimpl]
            impl B { pub fn b(_env: Env) {} }
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts.len(), 2);
        for c in &contracts {
            assert_eq!(
                c.storage_types.len(),
                1,
                "contract {} missing storage type",
                c.struct_name
            );
        }
    }

    // ── Multiple contracts ────────────────────────────────────────────────────

    #[test]
    fn two_contracts_in_one_file_are_each_discovered() {
        let file = file_from(
            r#"
            #[contract]
            pub struct TokenA;
            #[contractimpl]
            impl TokenA {
                pub fn name_a(_env: Env) -> u32 { 1 }
            }
            #[contract]
            pub struct TokenB;
            #[contractimpl]
            impl TokenB {
                pub fn name_b(_env: Env) -> u32 { 2 }
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        assert_eq!(contracts.len(), 2);
        let names: Vec<&str> = contracts.iter().map(|c| c.struct_name.as_str()).collect();
        assert!(names.contains(&"TokenA"));
        assert!(names.contains(&"TokenB"));
    }

    #[test]
    fn results_are_in_deterministic_order() {
        let file = file_from(
            r#"
            #[contract] pub struct Zebra;
            #[contractimpl] impl Zebra { pub fn z(_env: Env) {} }
            #[contract] pub struct Apple;
            #[contractimpl] impl Apple { pub fn a(_env: Env) {} }
        "#,
        );
        let contracts = discover_contracts(&file);
        // BTreeMap guarantees lexicographic order.
        assert_eq!(contracts[0].struct_name, "Apple");
        assert_eq!(contracts[1].struct_name, "Zebra");
    }

    // ── Line numbers ──────────────────────────────────────────────────────────

    #[test]
    fn function_line_numbers_are_positive() {
        let file = file_from(
            r#"
            #[contract]
            pub struct MyContract;
            #[contractimpl]
            impl MyContract {
                pub fn entry(_env: Env) {}
            }
        "#,
        );
        let contracts = discover_contracts(&file);
        let f = &contracts[0].fns[0];
        assert!(f.line > 0, "line must be > 0, got {}", f.line);
    }
}
