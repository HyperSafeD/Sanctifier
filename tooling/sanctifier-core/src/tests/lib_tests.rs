// tooling/sanctifier-core/src/tests/lib_tests.rs

use crate::*;

#[test]
fn test_analyze_with_macros() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        use soroban_sdk::{contract, contractimpl, Env};

        #[contract]
        pub struct MyContract;

        #[contractimpl]
        impl MyContract {
            pub fn hello(env: Env) {}
        }

        #[contracttype]
        pub struct SmallData {
            pub x: u32,
        }

        #[contracttype]
        pub struct BigData {
            pub buffer: Bytes,
            pub large: u128,
        }
    "#;
    let warnings = analyzer.analyze_ledger_size(source);
    // SmallData: 4 bytes — BigData: 64 + 16 = 80 bytes — both under 64 KB
    assert!(warnings.is_empty());
}

#[test]
fn test_analyze_with_limit() {
    let mut config = SanctifyConfig::default();
    config.ledger_limit = 50;
    let analyzer = Analyzer::new(config);
    let source = r#"
        #[contracttype]
        pub struct ExceedsLimit {
            pub buffer: Bytes, // 64 bytes estimated
        }
    "#;
    let warnings = analyzer.analyze_ledger_size(source);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].struct_name, "ExceedsLimit");
    assert_eq!(warnings[0].estimated_size, 64);
    assert_eq!(warnings[0].level, SizeWarningLevel::ExceedsLimit);
}

#[test]
fn test_ledger_size_enum_and_approaching() {
    let mut config = SanctifyConfig::default();
    config.ledger_limit = 100;
    config.approaching_threshold = 0.5;
    let analyzer = Analyzer::new(config);
    let source = r#"
        #[contracttype]
        pub enum DataKey {
            Balance(Address),
            Admin,
        }

        #[contracttype]
        pub struct NearLimit {
            pub a: u128,
            pub b: u128,
            pub c: u128,
            pub d: u128,
        }
    "#;
    let warnings = analyzer.analyze_ledger_size(source);
    assert!(warnings.iter().any(|w| w.struct_name == "NearLimit"), "NearLimit (64 bytes) should exceed 50% of 100");
    assert!(warnings.iter().any(|w| w.level == SizeWarningLevel::ApproachingLimit));
}

#[test]
fn test_complex_macro_no_panic() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        macro_rules! complex {
            ($($t:tt)*) => { $($t)* };
        }

        complex! {
            pub struct MyStruct {
                pub x: u32,
            }
        }

        #[contractimpl]
        impl Contract {
            pub fn test() {
                let x = symbol_short!("test");
            }
        }
    "#;
    let _ = analyzer.analyze_ledger_size(source);
}

#[test]
fn test_heavy_macro_usage_graceful() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        use soroban_sdk::{contract, contractimpl, Env};

        #[contract]
        pub struct Token;

        #[contractimpl]
        impl Token {
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                // Heavy macro expansion - analyzer must not panic
            }
        }
    "#;
    let _ = analyzer.scan_auth_gaps(source);
    let _ = analyzer.scan_panics(source);
    let _ = analyzer.analyze_unsafe_patterns(source);
    let _ = analyzer.analyze_ledger_size(source);
    let _ = analyzer.scan_arithmetic_overflow(source);
}

#[test]
fn test_scan_auth_gaps() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn set_data(env: Env, val: u32) {
                env.storage().instance().set(&DataKey::Val, &val);
            }

            pub fn set_data_secure(env: Env, val: u32) {
                env.require_auth();
                env.storage().instance().set(&DataKey::Val, &val);
            }

            pub fn get_data(env: Env) -> u32 {
                env.storage().instance().get(&DataKey::Val).unwrap_or(0)
            }

            pub fn no_storage(env: Env) {
                let x = 1 + 1;
            }
        }
    "#;
    let gaps = analyzer.scan_auth_gaps(source);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0], "set_data");
}

#[test]
fn test_scan_panics() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn unsafe_fn(env: Env) {
                panic!("Something went wrong");
            }

            pub fn unsafe_unwrap(env: Env) {
                let x: Option<u32> = None;
                let y = x.unwrap();
            }

            pub fn unsafe_expect(env: Env) {
                let x: Option<u32> = None;
                let y = x.expect("Failed to get x");
            }

            pub fn safe_fn(env: Env) -> Result<(), u32> {
                Ok(())
            }
        }
    "#;
    let issues = analyzer.scan_panics(source);
    assert_eq!(issues.len(), 3);

    let types: Vec<String> = issues.iter().map(|i| i.issue_type.clone()).collect();
    assert!(types.contains(&"panic!".to_string()));
    assert!(types.contains(&"unwrap".to_string()));
    assert!(types.contains(&"expect".to_string()));
}

#[test]
fn test_scan_arithmetic_overflow_basic() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn add_balances(env: Env, a: u64, b: u64) -> u64 {
                a + b
            }

            pub fn subtract(env: Env, total: u128, amount: u128) -> u128 {
                total - amount
            }

            pub fn multiply(env: Env, price: u64, qty: u64) -> u64 {
                price * qty
            }

            pub fn safe_add(env: Env, a: u64, b: u64) -> Option<u64> {
                a.checked_add(b)
            }
        }
    "#;
    let issues = analyzer.scan_arithmetic_overflow(source);
    // Three distinct (function, operator) pairs flagged
    assert_eq!(issues.len(), 3);

    let ops: Vec<&str> = issues.iter().map(|i| i.operation.as_str()).collect();
    assert!(ops.contains(&"+"));
    assert!(ops.contains(&"-"));
    assert!(ops.contains(&"*"));

    // safe_add uses checked_add — no bare + operator, so not flagged
    assert!(issues.iter().all(|i| i.function_name != "safe_add"));
}

#[test]
fn test_scan_arithmetic_overflow_compound_assign() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl Token {
            pub fn accumulate(env: Env, mut balance: u64, amount: u64) -> u64 {
                balance += amount;
                balance -= 1;
                balance *= 2;
                balance
            }
        }
    "#;
    let issues = analyzer.scan_arithmetic_overflow(source);
    // One issue per compound operator per function
    assert_eq!(issues.len(), 3);
    let ops: Vec<&str> = issues.iter().map(|i| i.operation.as_str()).collect();
    assert!(ops.contains(&"+="));
    assert!(ops.contains(&"-="));
    assert!(ops.contains(&"*="));
}

#[test]
fn test_scan_arithmetic_overflow_deduplication() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn sum_three(env: Env, a: u64, b: u64, c: u64) -> u64 {
                // Two `+` operations — should produce only ONE issue for this function
                a + b + c
            }
        }
    "#;
    let issues = analyzer.scan_arithmetic_overflow(source);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].operation, "+");
    assert_eq!(issues[0].function_name, "sum_three");
}

#[test]
fn test_scan_arithmetic_overflow_no_false_positive_safe_code() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn compare(env: Env, a: u64, b: u64) -> bool {
                a > b
            }

            pub fn bitwise(env: Env, a: u32) -> u32 {
                a & 0xFF
            }
        }
    "#;
    let issues = analyzer.scan_arithmetic_overflow(source);
    assert!(
        issues.is_empty(),
        "Expected no issues but found: {:?}",
        issues
    );
}

#[test]
fn test_scan_arithmetic_overflow_custom_wrapper_types() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    // Custom type wrapping a primitive — arithmetic on it is still flagged
    let source = r#"
        #[contractimpl]
        impl Vault {
            pub fn add_shares(env: Env, current: Shares, delta: Shares) -> Shares {
                Shares(current.0 + delta.0)
            }
        }
    "#;
    let issues = analyzer.scan_arithmetic_overflow(source);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].operation, "+");
}

#[test]
fn test_analyze_upgrade_patterns() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contracttype]
        pub enum DataKey { Admin, Balance }

        #[contractimpl]
        impl Token {
            pub fn initialize(env: Env, admin: Address) {
                env.storage().instance().set(&DataKey::Admin, &admin);
            }
            pub fn set_admin(env: Env, new_admin: Address) {
                env.storage().instance().set(&DataKey::Admin, &new_admin);
            }
        }
    "#;
    let report = analyzer.analyze_upgrade_patterns(source);
    assert_eq!(report.init_functions, vec!["initialize"]);
    assert_eq!(report.upgrade_mechanisms, vec!["set_admin"]);
    assert!(report.storage_types.contains(&"DataKey".to_string()));
    assert!(report
        .findings
        .iter()
        .any(|f| matches!(f.category, UpgradeCategory::Governance)));
}

#[test]
fn test_scan_arithmetic_overflow_suggestion_content() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        #[contractimpl]
        impl MyContract {
            pub fn risky(env: Env, a: u64, b: u64) -> u64 {
                a + b
            }
        }
    "#;
    let issues = analyzer.scan_arithmetic_overflow(source);
    assert_eq!(issues.len(), 1);
    // Suggestion should mention checked_add
    assert!(issues[0].suggestion.contains("checked_add"));
    // Location should include function name
    assert!(issues[0].location.starts_with("risky:"));
}

#[test]
fn test_scan_storage_collisions() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        const KEY1: &str = "collision";
        const KEY2: &str = "collision";
        
        #[contractimpl]
        impl Contract {
            pub fn x() {
                let s = symbol_short!("other");
                let s2 = symbol_short!("other");
            }
        }
    "#;
    let issues = analyzer.scan_storage_collisions(source);
    // 2 for "collision" (KEY1, KEY2) + 2 for "other" (two symbol_short! calls)
    assert!(issues.iter().any(|i| i.key_value == "collision"));
    assert!(issues.iter().any(|i| i.key_value == "other"));
}
