// tooling/sanctifier-core/src/tests/storage_collision_tests.rs

#[cfg(test)]
mod tests {
    use crate::{Analyzer, SanctifyConfig};

    fn analyzer() -> Analyzer {
        Analyzer::new(SanctifyConfig::default())
    }

    #[test]
    fn test_const_string_collision() {
        let src = r#"
            const KEY_A: &str = "collision";
            const KEY_B: &str = "collision";
        "#;
        let issues = analyzer().scan_storage_collisions(src);
        assert!(!issues.is_empty(), "Duplicate const string keys should be flagged");
        assert!(issues.iter().any(|i| i.key_value == "collision"));
    }

    #[test]
    fn test_symbol_short_collision() {
        let src = r#"
            #[contractimpl]
            impl Contract {
                pub fn a() {
                    let _s1 = symbol_short!("tok");
                }
                pub fn b() {
                    let _s2 = symbol_short!("tok");
                }
            }
        "#;
        let issues = analyzer().scan_storage_collisions(src);
        assert!(!issues.is_empty(), "Duplicate symbol_short! should be flagged");
        assert!(issues.iter().any(|i| i.key_value == "\"tok\"" || i.key_value == "tok"));
    }

    #[test]
    fn test_no_collision_unique_keys() {
        let src = r#"
            const KEY_A: &str = "alpha";
            const KEY_B: &str = "beta";

            #[contractimpl]
            impl Contract {
                pub fn x() {
                    let _s = symbol_short!("gamma");
                }
            }
        "#;
        let issues = analyzer().scan_storage_collisions(src);
        assert!(issues.is_empty(), "All unique keys — no collisions expected");
    }

    #[test]
    fn test_symbol_new_collision() {
        let src = r#"
            #[contractimpl]
            impl Contract {
                pub fn a(env: Env) {
                    let _k = Symbol::new(&env, "shared_key");
                }
                pub fn b(env: Env) {
                    let _k = Symbol::new(&env, "shared_key");
                }
            }
        "#;
        let issues = analyzer().scan_storage_collisions(src);
        assert!(!issues.is_empty(), "Duplicate Symbol::new keys should be flagged");
        assert!(issues.iter().any(|i| i.key_value == "shared_key"));
    }

    #[test]
    fn test_invalid_source_returns_empty() {
        let issues = analyzer().scan_storage_collisions("not valid rust !!!");
        assert!(issues.is_empty());
    }
}
