use sanctifier_core::Analyzer;
use std::fs;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("fixture should be readable")
}

#[test]
fn auth_gap_fixture_emits_exactly_one_auth_gap() {
    let analyzer = Analyzer::new(Default::default());
    let source = read_fixture("auth_gap_contract.rs");

    let findings = analyzer.scan_auth_gaps(&source);

    assert_eq!(findings, vec!["store_user".to_string()]);
}

#[test]
fn clean_token_fixture_has_zero_findings() {
    let analyzer = Analyzer::new(Default::default());
    let source = read_fixture("clean_token.rs");

    assert!(analyzer.scan_auth_gaps(&source).is_empty());
    assert!(analyzer.scan_panics(&source).is_empty());
    assert!(analyzer.scan_arithmetic_overflow(&source).is_empty());
    assert!(analyzer.verify_sep41_interface(&source).issues.is_empty());
}

#[test]
fn overflow_fixture_emits_single_arithmetic_issue() {
    let analyzer = Analyzer::new(Default::default());
    let source = read_fixture("overflow_contract.rs");

    let issues = analyzer.scan_arithmetic_overflow(&source);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].operation, "+");
}

#[test]
fn reentrancy_fixture_contains_cross_contract_edge() {
    let analyzer = Analyzer::new(Default::default());
    let source = read_fixture("reentrancy_contract.rs");

    let edges = analyzer.scan_invoke_contract_calls(
        &source,
        "ReentrancyContract",
        "tests/fixtures/reentrancy_contract.rs",
    );

    assert_eq!(edges.len(), 1);
    assert!(edges[0].function_expr.as_ref().is_some());
}
