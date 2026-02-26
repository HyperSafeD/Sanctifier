// tooling/sanctifier-core/src/tests/complexity_tests.rs

#[cfg(test)]
mod tests {
    use crate::complexity::analyze_complexity;
    use syn::parse_file;

    fn parse(src: &str) -> syn::File {
        parse_file(src).expect("should parse")
    }

    #[test]
    fn test_simple_fn_has_cc_of_one() {
        let src = r#"
            pub fn get_balance(env: Env, addr: Address) -> i128 {
                env.storage().persistent().get(&addr).unwrap_or(0)
            }
        "#;
        let metrics = analyze_complexity(&parse(src), "test.rs");
        let f = &metrics.functions[0];
        assert_eq!(f.cyclomatic_complexity, 1);
        assert!(f.warnings.is_empty());
    }

    #[test]
    fn test_if_increments_cc() {
        let src = r#"
            pub fn transfer(env: Env, amount: i128) {
                if amount > 0 {
                    env.storage().persistent().set(&"k", &amount);
                }
            }
        "#;
        let metrics = analyze_complexity(&parse(src), "test.rs");
        let f = &metrics.functions[0];
        assert_eq!(f.cyclomatic_complexity, 2);
    }

    #[test]
    fn test_nesting_depth_tracked() {
        let src = r#"
            pub fn nested(env: Env, x: i128) -> i128 {
                if x > 0 {
                    for i in 0..x {
                        if i > 5 {
                            return i;
                        }
                    }
                }
                0
            }
        "#;
        let metrics = analyze_complexity(&parse(src), "test.rs");
        let f = &metrics.functions[0];
        assert!(f.max_nesting_depth >= 3);
    }

    #[test]
    fn test_param_count() {
        let src = r#"
            pub fn many_params(env: Env, a: i128, b: i128, c: i128, d: i128, e: i128) {}
        "#;
        let metrics = analyze_complexity(&parse(src), "test.rs");
        let f = &metrics.functions[0];
        assert_eq!(f.param_count, 7); // env + 6
        assert!(f.warnings.iter().any(|w| w.contains("parameters")));
    }

    #[test]
    fn test_use_counts_as_dependency() {
        let src = r#"
            use soroban_sdk::{Env, Address};
            use soroban_sdk::token::TokenClient;
            pub fn foo(env: Env) {}
        "#;
        let metrics = analyze_complexity(&parse(src), "test.rs");
        assert_eq!(metrics.dependency_count, 2);
    }

    #[test]
    fn test_match_arms_add_to_cc() {
        let src = r#"
            pub fn route(env: Env, action: u32) {
                match action {
                    0 => {},
                    1 => {},
                    2 => {},
                    _ => {},
                }
            }
        "#;
        let metrics = analyze_complexity(&parse(src), "test.rs");
        let f = &metrics.functions[0];
        // 1 base + 3 extra arms
        assert_eq!(f.cyclomatic_complexity, 4);
    }
}