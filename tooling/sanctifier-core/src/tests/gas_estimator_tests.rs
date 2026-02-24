// tooling/sanctifier-core/src/tests/gas_estimator_tests.rs

#[cfg(test)]
mod tests {
    use crate::gas_estimator::GasEstimator;

    fn estimate(src: &str) -> Vec<crate::gas_estimator::GasEstimationReport> {
        GasEstimator::new().estimate_contract(src)
    }

    #[test]
    fn test_simple_fn_baseline() {
        let src = r#"
            #[contractimpl]
            impl Token {
                pub fn get_balance(env: Env, addr: Address) -> i128 {
                    env.storage().persistent().get(&addr).unwrap_or(0)
                }
            }
        "#;
        let reports = estimate(src);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].function_name, "get_balance");
        // Base (50) + at least 1 method call (25+)
        assert!(reports[0].estimated_instructions > 50);
    }

    #[test]
    fn test_storage_ops_are_expensive() {
        let src = r#"
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    env.storage().persistent().set(&from, &amount);
                    env.storage().persistent().set(&to, &amount);
                }
            }
        "#;
        let reports = estimate(src);
        assert_eq!(reports.len(), 1);
        // Two `.set()` calls at 1000 each
        assert!(reports[0].estimated_instructions >= 2000);
    }

    #[test]
    fn test_loop_multiplies_cost() {
        let src = r#"
            #[contractimpl]
            impl Token {
                pub fn batch(env: Env, count: u32) {
                    for _ in 0..count {
                        env.storage().persistent().set(&count, &count);
                    }
                }
            }
        "#;
        let loop_reports = estimate(src);

        let no_loop_src = r#"
            #[contractimpl]
            impl Token {
                pub fn single(env: Env, count: u32) {
                    env.storage().persistent().set(&count, &count);
                }
            }
        "#;
        let no_loop_reports = estimate(no_loop_src);
        // Loop version should have more estimated instructions
        assert!(
            loop_reports[0].estimated_instructions > no_loop_reports[0].estimated_instructions,
            "Loop should increase instruction estimate"
        );
    }

    #[test]
    fn test_invalid_source_returns_empty() {
        let reports = estimate("this is not valid rust code !!!");
        assert!(reports.is_empty());
    }

    #[test]
    fn test_non_public_fns_ignored() {
        let src = r#"
            #[contractimpl]
            impl Token {
                fn internal_helper(env: Env) -> i128 {
                    env.storage().persistent().get(&0u32).unwrap_or(0)
                }
            }
        "#;
        let reports = estimate(src);
        // Private functions are not reported
        assert!(reports.is_empty());
    }
}
