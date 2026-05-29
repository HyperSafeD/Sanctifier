//! Benchmark tests for the SEP-41 token contract (`my-contract`).
//!
//! Each test verifies that the named operation completes within the Soroban
//! host's default resource budget.  A test failure means the operation either
//! panicked or exceeded the host-enforced CPU / memory ceiling.
//!
//! Expected ceilings are documented in [`crate::budgets`].

#[cfg(test)]
mod tests {
    use my_contract::{Token, TokenClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn setup(env: &Env) -> (TokenClient<'_>, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Token);
        let client = TokenClient::new(env, &id);
        env.mock_all_auths();
        client.initialize(
            &admin,
            &7u32,
            &String::from_str(env, "BenchToken"),
            &String::from_str(env, "BT"),
        );
        (client, admin)
    }

    // -----------------------------------------------------------------------
    // Initialisation
    // -----------------------------------------------------------------------

    #[test]
    fn initialize_within_budget() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();
        let id = env.register_contract(None, Token);
        let client = TokenClient::new(&env, &id);
        client.initialize(
            &admin,
            &7u32,
            &String::from_str(&env, "BenchToken"),
            &String::from_str(&env, "BT"),
        );
    }

    // -----------------------------------------------------------------------
    // Mint
    // -----------------------------------------------------------------------

    #[test]
    fn mint_small_amount_within_budget() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let alice = Address::generate(&env);
        client.mint(&alice, &1_000i128);
    }

    #[test]
    fn mint_large_amount_within_budget() {
        let env = Env::default();
        let (client, _) = setup(&env);
        let alice = Address::generate(&env);
        client.mint(&alice, &(i128::MAX / 2));
    }

    // -----------------------------------------------------------------------
    // Transfer
    // -----------------------------------------------------------------------

    #[test]
    fn transfer_within_budget() {
        let env = Env::default();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let (client, _) = setup(&env);
        client.mint(&alice, &1_000_000i128);
        client.transfer(&alice, &bob, &500_000i128);
    }

    #[test]
    fn ten_sequential_transfers_within_budget() {
        let env = Env::default();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let (client, _) = setup(&env);
        client.mint(&alice, &10_000i128);
        for _ in 0..10 {
            client.transfer(&alice, &bob, &100i128);
            client.transfer(&bob, &alice, &100i128);
        }
    }

    // -----------------------------------------------------------------------
    // Approve / transfer_from
    // -----------------------------------------------------------------------

    #[test]
    fn approve_within_budget() {
        let env = Env::default();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let (client, _) = setup(&env);
        client.approve(&alice, &bob, &500i128, &1_000u32);
    }

    #[test]
    fn transfer_from_within_budget() {
        let env = Env::default();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let carol = Address::generate(&env);
        let (client, _) = setup(&env);
        client.mint(&alice, &1_000i128);
        client.approve(&alice, &bob, &500i128, &1_000u32);
        client.transfer_from(&bob, &alice, &carol, &300i128);
    }

    // -----------------------------------------------------------------------
    // Burn
    // -----------------------------------------------------------------------

    #[test]
    fn burn_within_budget() {
        let env = Env::default();
        let alice = Address::generate(&env);
        let (client, _) = setup(&env);
        client.mint(&alice, &1_000i128);
        client.burn(&alice, &400i128);
    }

    #[test]
    fn burn_from_within_budget() {
        let env = Env::default();
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let (client, _) = setup(&env);
        client.mint(&alice, &1_000i128);
        client.approve(&alice, &bob, &500i128, &1_000u32);
        client.burn_from(&bob, &alice, &200i128);
    }
}
