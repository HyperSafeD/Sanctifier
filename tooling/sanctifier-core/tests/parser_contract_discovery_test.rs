//! Integration tests for `parser` + `contract_discovery`.
//!
//! These tests run against the canonical fixture files in `tests/fixtures/`
//! and verify end-to-end behaviour: read raw source → validate → parse → discover.

use sanctifier_core::{contract_discovery, parser};
use std::fs;
use std::path::PathBuf;

// ── Fixture helpers ────────────────────────────────────────────────────────────

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|_| panic!("fixture '{name}' should be readable"))
}

// ── parser::parse_source ───────────────────────────────────────────────────────

#[test]
fn parse_source_succeeds_on_all_fixtures() {
    for fixture in &[
        "minimal_contract.rs",
        "auth_gap_contract.rs",
        "clean_token.rs",
        "overflow_contract.rs",
        "reentrancy_contract.rs",
        "multi_contract.rs",
        "contract_with_storage_types.rs",
    ] {
        let source = read_fixture(fixture);
        assert!(
            parser::parse_source(&source).is_ok(),
            "parse_source failed on fixture: {fixture}"
        );
    }
}

#[test]
fn parse_source_rejects_empty_string() {
    let result = parser::parse_source("");
    assert!(
        matches!(result, Err(parser::ParseError::Validation(_))),
        "expected Validation error for empty string"
    );
}

#[test]
fn parse_source_rejects_null_bytes() {
    let result = parser::parse_source("fn foo() { let _ = \0; }");
    assert!(
        matches!(result, Err(parser::ParseError::Validation(_))),
        "expected Validation error for null-byte input"
    );
}

#[test]
fn parse_source_rejects_invalid_rust_syntax() {
    let result = parser::parse_source("this {{ is not rust");
    assert!(
        matches!(result, Err(parser::ParseError::Syntax(_))),
        "expected Syntax error for malformed input"
    );
}

#[test]
fn parse_error_has_non_empty_display() {
    let e = parser::parse_source("").unwrap_err();
    assert!(!e.to_string().is_empty());
}

// ── contract_discovery — minimal_contract.rs ──────────────────────────────────

#[test]
fn minimal_contract_fixture_discovers_one_contract() {
    let source = read_fixture("minimal_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    assert_eq!(contracts.len(), 1);
    let c = &contracts[0];
    assert_eq!(c.struct_name, "MinimalContract");
    assert!(
        c.has_contract_attr,
        "MinimalContract must carry #[contract]"
    );
    assert!(
        c.has_contractimpl,
        "MinimalContract must have a #[contractimpl] block"
    );
}

#[test]
fn minimal_contract_exposes_exactly_one_public_function() {
    let source = read_fixture("minimal_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    let fns: Vec<_> = contracts[0].public_functions().collect();
    assert_eq!(
        fns.len(),
        1,
        "expected exactly one non-reserved public function"
    );
    assert_eq!(fns[0].name, "ping");
    assert!(!fns[0].is_reserved);
}

// ── contract_discovery — auth_gap_contract.rs ─────────────────────────────────

#[test]
fn auth_gap_fixture_discovers_auth_gap_contract() {
    let source = read_fixture("auth_gap_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].struct_name, "AuthGapContract");
}

#[test]
fn auth_gap_fixture_discovers_two_public_functions() {
    let source = read_fixture("auth_gap_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    let fns: Vec<_> = contracts[0].public_functions().collect();
    assert_eq!(fns.len(), 2, "expected store_user and has_user");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"store_user"));
    assert!(names.contains(&"has_user"));
}

// ── contract_discovery — multi_contract.rs ────────────────────────────────────

#[test]
fn multi_contract_fixture_discovers_two_contracts() {
    let source = read_fixture("multi_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    assert_eq!(contracts.len(), 2);
    let names: Vec<&str> = contracts.iter().map(|c| c.struct_name.as_str()).collect();
    assert!(names.contains(&"TokenA"), "TokenA not found in {names:?}");
    assert!(names.contains(&"VaultB"), "VaultB not found in {names:?}");
}

#[test]
fn multi_contract_fixture_storage_types_on_each_contract() {
    let source = read_fixture("multi_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    for c in &contracts {
        assert_eq!(
            c.storage_types.len(),
            2,
            "contract {} should see both contracttype items",
            c.struct_name
        );
        let type_names: Vec<&str> = c.storage_types.iter().map(|t| t.name.as_str()).collect();
        assert!(type_names.contains(&"TokenKey"));
        assert!(type_names.contains(&"VaultKey"));
    }
}

#[test]
fn multi_contract_token_a_has_three_public_functions() {
    let source = read_fixture("multi_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    let token_a = contracts
        .iter()
        .find(|c| c.struct_name == "TokenA")
        .unwrap();
    let fns: Vec<_> = token_a.public_functions().collect();
    assert_eq!(fns.len(), 3, "expected initialize, balance, transfer");
}

#[test]
fn multi_contract_vault_b_has_two_public_functions() {
    let source = read_fixture("multi_contract.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    let vault = contracts
        .iter()
        .find(|c| c.struct_name == "VaultB")
        .unwrap();
    let fns: Vec<_> = vault.public_functions().collect();
    assert_eq!(fns.len(), 2, "expected deposit and total");
}

// ── contract_discovery — contract_with_storage_types.rs ──────────────────────

#[test]
fn storage_types_fixture_discovers_one_contract() {
    let source = read_fixture("contract_with_storage_types.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].struct_name, "Registry");
}

#[test]
fn storage_types_fixture_collects_both_contracttype_items() {
    let source = read_fixture("contract_with_storage_types.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    assert_eq!(contracts[0].storage_types.len(), 2);
    let type_names: Vec<&str> = contracts[0]
        .storage_types
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert!(
        type_names.contains(&"DataKey"),
        "DataKey missing from {type_names:?}"
    );
    assert!(
        type_names.contains(&"Config"),
        "Config missing from {type_names:?}"
    );
}

#[test]
fn storage_types_fixture_constructor_is_reserved() {
    let source = read_fixture("contract_with_storage_types.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    let all = contracts[0].all_public_functions();
    let ctor = all.iter().find(|f| f.name == "__constructor").unwrap();
    assert!(ctor.is_reserved, "__constructor must be marked reserved");
}

#[test]
fn storage_types_fixture_non_reserved_public_functions() {
    let source = read_fixture("contract_with_storage_types.rs");
    let file = parser::parse_source(&source).unwrap().file;
    let contracts = contract_discovery::discover_contracts(&file);

    let fns: Vec<_> = contracts[0].public_functions().collect();
    assert_eq!(fns.len(), 3, "expected increment, get_count, admin");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"increment"));
    assert!(names.contains(&"get_count"));
    assert!(names.contains(&"admin"));
}

// ── Line number sanity ────────────────────────────────────────────────────────

#[test]
fn all_fixture_functions_have_positive_line_numbers() {
    for fixture in &[
        "minimal_contract.rs",
        "auth_gap_contract.rs",
        "multi_contract.rs",
        "contract_with_storage_types.rs",
    ] {
        let source = read_fixture(fixture);
        let file = parser::parse_source(&source).unwrap().file;
        let contracts = contract_discovery::discover_contracts(&file);
        for c in &contracts {
            for f in c.all_public_functions() {
                assert!(
                    f.line > 0,
                    "fixture {fixture}: function '{}' has line 0",
                    f.name
                );
            }
        }
    }
}
