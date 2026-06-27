//! Helpers for deploying a Stellar Asset Contract (SAC) in tests.

use soroban_sdk::{testutils::Address as _, token, Address, Env};

/// Deploy a Stellar Asset Contract and mint `amount` units to `recipient`.
///
/// Returns the token contract address.  Callers can wrap it with
/// `token::Client::new(&env, &addr)` for further operations.
///
/// # Example
///
/// ```rust,ignore
/// use sanctifier_test_support::{env::TestEnv, token::create_token};
///
/// let te = TestEnv::new();
/// let user = Address::generate(&te.env);
/// let token_id = create_token(&te.env, &user, 1_000_000);
/// let client = soroban_sdk::token::Client::new(&te.env, &token_id);
/// assert_eq!(client.balance(&user), 1_000_000);
/// ```
pub fn create_token(env: &Env, recipient: &Address, amount: i128) -> Address {
    let admin = Address::generate(env);
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let sac = token::StellarAssetClient::new(env, &token_addr);
    sac.mint(recipient, &amount);
    token_addr
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn create_token_mints_expected_amount() {
        let env = Env::default();
        env.mock_all_auths();
        let user = Address::generate(&env);
        let token_id = create_token(&env, &user, 500);
        let client = token::Client::new(&env, &token_id);
        assert_eq!(client.balance(&user), 500);
    }

    #[test]
    fn create_token_returns_unique_addresses_across_calls() {
        let env = Env::default();
        env.mock_all_auths();
        let user = Address::generate(&env);
        let a = create_token(&env, &user, 100);
        let b = create_token(&env, &user, 100);
        assert_ne!(a, b);
    }
}
