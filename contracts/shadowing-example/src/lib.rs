#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol, symbol_short};

/// Example contract demonstrating variable shadowing issues.
///
/// This contract intentionally contains variable shadowing patterns
/// that the Sanctifier tool should detect.
#[contract]
pub struct ShadowingExample;

#[contractimpl]
impl ShadowingExample {
    /// Example 1: Simple shadowing in nested block
    /// BUG: The inner `balance` shadows the outer one, potentially causing confusion
    pub fn update_balance_buggy(env: Env, user: Symbol, amount: i128) -> i128 {
        let balance = Self::get_balance(env.clone(), user.clone());
        
        // This block shadows the outer balance variable
        {
            let balance = amount;  // SHADOWING: This shadows the outer balance!
            env.storage().persistent().set(&user, &balance);
        }
        
        // Developer might think they're returning the updated balance,
        // but they're actually returning the original balance
        balance  // BUG: Returns the OLD balance, not the new one!
    }

    /// Example 2: Shadowing in conditional blocks
    /// BUG: Different branches shadow the same variable
    pub fn process_transaction_buggy(env: Env, amount: i128, fee: i128) -> i128 {
        let total = amount;
        
        if fee > 0 {
            let total = amount + fee;  // SHADOWING: Shadows outer total
            env.storage().instance().set(&symbol_short!("last_tx"), &total);
        } else {
            let total = amount;  // SHADOWING: Also shadows outer total
            env.storage().instance().set(&symbol_short!("last_tx"), &total);
        }
        
        // Returns the original total, not the calculated one
        total  // BUG: Returns amount, not amount + fee
    }

    /// Example 3: Shadowing in for loop
    /// BUG: Loop variable shadows outer variable
    pub fn calculate_sum_buggy(env: Env, count: u32) -> u32 {
        let i = 100;  // Some important value
        let mut sum = 0;
        
        for i in 0..count {  // SHADOWING: Loop variable shadows outer i
            sum += i;
        }
        
        // Developer might expect i to still be 100 here
        env.storage().instance().set(&symbol_short!("last_i"), &i);
        sum
    }

    /// Example 4: Shadowing in match arms
    /// BUG: Match arm pattern shadows outer variable
    pub fn handle_option_buggy(env: Env, opt: Option<i128>) -> i128 {
        let value = 42;  // Default value
        
        match opt {
            Some(value) => {  // SHADOWING: Pattern binding shadows outer value
                env.storage().instance().set(&symbol_short!("opt_val"), &value);
                value
            }
            None => value,  // Uses outer value
        }
        // The outer value is never modified, which might be unexpected
    }

    /// Example 5: Correct implementation without shadowing
    pub fn update_balance_correct(env: Env, user: Symbol, amount: i128) -> i128 {
        let old_balance = Self::get_balance(env.clone(), user.clone());
        
        {
            let new_balance = amount;  // Different name, no shadowing
            env.storage().persistent().set(&user, &new_balance);
        }
        
        amount  // Correctly returns the new balance
    }

    /// Example 6: Correct implementation with explicit naming
    pub fn process_transaction_correct(env: Env, amount: i128, fee: i128) -> i128 {
        let base_amount = amount;
        
        let final_total = if fee > 0 {
            let total_with_fee = amount + fee;  // Clear, distinct name
            env.storage().instance().set(&symbol_short!("last_tx"), &total_with_fee);
            total_with_fee
        } else {
            let total_no_fee = amount;  // Clear, distinct name
            env.storage().instance().set(&symbol_short!("last_tx"), &total_no_fee);
            total_no_fee
        };
        
        final_total
    }

    // Helper function
    fn get_balance(env: Env, user: Symbol) -> i128 {
        env.storage().persistent().get(&user).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_shadowing_bug_demonstration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ShadowingExample);
        let client = ShadowingExampleClient::new(&env, &contract_id);

        let user = symbol_short!("alice");
        
        // The buggy version returns the OLD balance (0), not the new one (100)
        let result = client.update_balance_buggy(&user, &100);
        assert_eq!(result, 0);  // BUG: Should be 100, but returns 0!
        
        // The correct version returns the new balance
        let result = client.update_balance_correct(&user, &100);
        assert_eq!(result, 100);  // Correct!
    }

    #[test]
    fn test_transaction_shadowing_bug() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ShadowingExample);
        let client = ShadowingExampleClient::new(&env, &contract_id);

        // Buggy version returns amount, not amount + fee
        let result = client.process_transaction_buggy(&100, &10);
        assert_eq!(result, 100);  // BUG: Should be 110, but returns 100!
        
        // Correct version returns the right total
        let result = client.process_transaction_correct(&100, &10);
        assert_eq!(result, 110);  // Correct!
    }
}
