use sanctifier_core::{Analyzer, PatternType, SanctifyConfig};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_token_integration_auth_and_panic() {
    let mut analyzer = Analyzer::new(SanctifyConfig::default());

    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push("tests/fixtures/vulnerable_token.rs");

    let source = fs::read_to_string(&fixture_path).expect("Failed to read vulnerable_token.rs");

    let missing_auth_funcs = analyzer.scan_auth_gaps(&source);

    // `mint` and `transfer` are missing require_auth. `burn` has it. `initialize` has it.
    assert!(
        missing_auth_funcs.contains(&"mint".to_string()),
        "mint function should flag missing auth"
    );
    assert!(
        missing_auth_funcs.contains(&"transfer".to_string()),
        "transfer function should flag missing auth"
    );
    assert!(
        !missing_auth_funcs.contains(&"burn".to_string()),
        "burn function should NOT flag missing auth, as it is present"
    );

    // Also check for panic or standard unsafety
    let unsafe_patterns = analyzer.analyze_unsafe_patterns(&source);
    assert!(
        unsafe_patterns
            .iter()
            .any(|p| matches!(p.pattern_type, PatternType::Panic)),
        "Analyzer should have detected the panic! in transfer"
    );
    assert!(
        unsafe_patterns
            .iter()
            .any(|p| matches!(p.pattern_type, PatternType::Unwrap)),
        "Analyzer should have detected the unwrap in burn or read_balance"
    );
}
