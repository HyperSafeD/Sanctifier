// tooling/sanctifier-core/src/tests/reentrancy_tests.rs

#[cfg(test)]
mod tests {
    use crate::reentrancy::scan_reentrancy;

    #[test]
    fn test_cei_violation_detected() {
        let src = r#"
            #[contractimpl]
            impl Vault {
                pub fn withdraw(env: Env, to: Address, amount: i128) {
                    // Interaction before Effect — CEI violation
                    let client = TokenClient::new(&env, &to);
                    client.transfer(&env.current_contract_address(), &to, &amount);
                    // Storage write after cross-contract call
                    env.storage().persistent().set(&DataKey::Balance, &0i128);
                }
            }
        "#;
        let issues = scan_reentrancy(src);
        assert!(!issues.is_empty(), "CEI violation should be detected");
        assert!(issues[0].function_name == "withdraw");
    }

    #[test]
    fn test_no_false_positive_on_read_then_call() {
        let src = r#"
            #[contractimpl]
            impl Vault {
                pub fn read_and_call(env: Env, to: Address) -> i128 {
                    // Read-only storage access then cross-contract call is fine
                    let balance: i128 = env.storage().persistent().get(&DataKey::Balance).unwrap_or(0);
                    let client = TokenClient::new(&env, &to);
                    client.get_balance(&to)
                }
            }
        "#;
        let issues = scan_reentrancy(src);
        assert!(issues.is_empty(), "Read then cross-call should not be flagged");
    }

    #[test]
    fn test_correct_cei_no_issue() {
        let src = r#"
            #[contractimpl]
            impl Token {
                pub fn safe_transfer(env: Env, from: Address, to: Address, amount: i128) {
                    // Check
                    let balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
                    assert!(balance >= amount);
                    // Effect (storage write first)
                    env.storage().persistent().set(&from, &(balance - amount));
                    // Interaction (cross-contract call last)
                    let client = TokenClient::new(&env, &to);
                    client.transfer(&env.current_contract_address(), &to, &amount);
                }
            }
        "#;
        let issues = scan_reentrancy(src);
        assert!(issues.is_empty(), "Correct CEI pattern should not be flagged");
    }

    #[test]
    fn test_multiple_functions_isolated() {
        let src = r#"
            #[contractimpl]
            impl Vault {
                pub fn bad_fn(env: Env, to: Address, amount: i128) {
                    let c = TokenClient::new(&env, &to);
                    c.transfer(&env.current_contract_address(), &to, &amount);
                    env.storage().persistent().set(&DataKey::Balance, &0i128);
                }
                pub fn good_fn(env: Env, to: Address, amount: i128) {
                    env.storage().persistent().set(&DataKey::Balance, &amount);
                    let c = TokenClient::new(&env, &to);
                    c.transfer(&env.current_contract_address(), &to, &amount);
                }
            }
        "#;
        let issues = scan_reentrancy(src);
        assert_eq!(issues.len(), 1, "Only bad_fn should be flagged");
        assert_eq!(issues[0].function_name, "bad_fn");
    }
}
