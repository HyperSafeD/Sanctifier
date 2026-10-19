//! Unit tests for the S007 custom rules execution pass (UX/DX improvements).
//!
//! Covers:
//! * `validate_custom_rules` surfaces invalid-regex errors with clear messages.
//! * `validate_custom_rules` flags empty rule names and empty patterns.
//! * Valid rules pass validation without errors.
//! * `analyze_custom_rules` matches at the correct line numbers.
//! * Overlapping rules each report their own matches independently.
//! * Multiple matches for one rule across many lines are all returned.
//! * Default `RuleSeverity` is honoured.
//! * Regex special characters in patterns work correctly.

use sanctifier_core::{
    Analyzer, CustomRule, CustomRuleValidationError, RuleSeverity, SanctifyConfig,
};

fn analyzer() -> Analyzer {
    Analyzer::new(SanctifyConfig::default())
}

/// Build an analyzer whose config carries the given custom rules, since
/// `analyze_custom_rules` now reads rules from the analyzer's config.
fn analyzer_with_rules(rules: Vec<CustomRule>) -> Analyzer {
    let config = SanctifyConfig {
        rules,
        ..SanctifyConfig::default()
    };
    Analyzer::new(config)
}

fn rule(name: &str, pattern: &str) -> CustomRule {
    CustomRule {
        name: name.to_string(),
        pattern: pattern.to_string(),
        description: String::new(),
        severity: RuleSeverity::Low,
    }
}

fn rule_with_severity(name: &str, pattern: &str, severity: RuleSeverity) -> CustomRule {
    CustomRule {
        name: name.to_string(),
        pattern: pattern.to_string(),
        description: String::new(),
        severity,
    }
}

// ── validate_custom_rules ────────────────────────────────────────────────────

#[test]
fn valid_rules_pass_validation() {
    let rules = vec![
        rule("no_panic", r"panic!"),
        rule("no_unwrap", r"\.unwrap\(\)"),
        rule("todo_marker", r"TODO"),
    ];
    let errors = analyzer().validate_custom_rules(&rules);
    assert!(
        errors.is_empty(),
        "valid rules should produce no errors; got: {errors:?}"
    );
}

#[test]
fn invalid_regex_is_reported_with_rule_name() {
    let rules = vec![rule("bad_regex", r"[unclosed")];
    let errors = analyzer().validate_custom_rules(&rules);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].rule_name, "bad_regex");
    assert!(
        errors[0].message.contains("invalid regex"),
        "error message should mention 'invalid regex'; got: {}",
        errors[0].message
    );
}

#[test]
fn empty_pattern_produces_validation_error() {
    let rules = vec![rule("empty", "")];
    let errors = analyzer().validate_custom_rules(&rules);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.contains("empty"),
        "error should mention empty pattern; got: {}",
        errors[0].message
    );
}

#[test]
fn empty_rule_name_produces_validation_error() {
    let rules = vec![rule("", r"panic!")];
    let errors = analyzer().validate_custom_rules(&rules);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.contains("name"),
        "error should mention name; got: {}",
        errors[0].message
    );
}

#[test]
fn multiple_invalid_rules_all_reported() {
    let rules = vec![
        rule("bad1", r"[bad"),
        rule("bad2", r"(unclosed"),
        rule("good", r"safe_pattern"),
    ];
    let errors = analyzer().validate_custom_rules(&rules);
    assert_eq!(errors.len(), 2, "both invalid rules should be reported");
    let names: Vec<&str> = errors.iter().map(|e| e.rule_name.as_str()).collect();
    assert!(names.contains(&"bad1"));
    assert!(names.contains(&"bad2"));
}

#[test]
fn validation_error_display_includes_rule_name_and_message() {
    let err = CustomRuleValidationError {
        rule_name: "my_rule".to_string(),
        message: "invalid regex pattern '[bad': ...".to_string(),
    };
    let display = format!("{err}");
    assert!(
        display.contains("my_rule"),
        "display should include rule name"
    );
    assert!(
        display.contains("invalid regex"),
        "display should include message"
    );
}

// ── analyze_custom_rules ─────────────────────────────────────────────────────

#[test]
fn rule_matches_at_correct_line_number() {
    let rules = vec![rule("find_todo", r"TODO")];
    let source = "fn a() {}\n// TODO: fix this\nfn b() {}";
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line, 2, "TODO is on line 2");
    assert_eq!(matches[0].rule_name, "find_todo");
}

#[test]
fn multiple_matches_for_one_rule_all_returned() {
    let rules = vec![rule("find_unwrap", r"\.unwrap\(\)")];
    let source = "let x = foo().unwrap();\nlet y = bar().unwrap();\nlet z = baz();";
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    assert_eq!(matches.len(), 2, "both unwrap lines should match");
}

#[test]
fn overlapping_rules_report_independently() {
    let rules = vec![
        rule("unsafe_kw", r"\bunsafe\b"),
        rule("unsafe_fn", r"unsafe fn"),
    ];
    let source = "pub unsafe fn danger() {}";
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    // Both rules match; they are independent findings.
    assert_eq!(matches.len(), 2);
    let names: Vec<&str> = matches.iter().map(|m| m.rule_name.as_str()).collect();
    assert!(names.contains(&"unsafe_kw"));
    assert!(names.contains(&"unsafe_fn"));
}

#[test]
fn no_match_returns_empty_vec() {
    let rules = vec![rule("find_never", r"THIS_NEVER_APPEARS_XYZ_123")];
    let source = "fn clean() { let x = 1; }";
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    assert!(matches.is_empty());
}

#[test]
fn severity_is_propagated_to_match() {
    let rules = vec![rule_with_severity(
        "critical_rule",
        r"panic!",
        RuleSeverity::Critical,
    )];
    let source = "panic!(\"oh no\");";
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].severity, RuleSeverity::Critical);
}

#[test]
fn snippet_is_trimmed_in_match() {
    let rules = vec![rule("find_set", r"\.set\(")];
    let source = "    env.storage().instance().set(&key, &val);";
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    assert_eq!(matches.len(), 1);
    // snippet should not start/end with whitespace
    assert_eq!(
        matches[0].snippet,
        matches[0].snippet.trim(),
        "snippet must be trimmed"
    );
}

#[test]
fn invalid_regex_in_rule_does_not_panic_or_affect_valid_rules() {
    let rules = vec![rule("bad", r"[unclosed"), rule("good", r"fn ")];
    let source = "fn hello() {}";
    // Should not panic; valid rule still matches.
    let matches = analyzer_with_rules(rules).analyze_custom_rules(source);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].rule_name, "good");
}

#[test]
fn empty_source_produces_no_matches() {
    let rules = vec![rule("any", r".+")];
    let matches = analyzer_with_rules(rules).analyze_custom_rules("");
    assert!(matches.is_empty());
}

#[test]
fn empty_rules_slice_produces_no_matches() {
    let source = "fn hello() { panic!(\"test\"); }";
    let matches = analyzer_with_rules(vec![]).analyze_custom_rules(source);
    assert!(matches.is_empty());
}
