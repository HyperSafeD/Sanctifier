#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Vec};

/// Example contract demonstrating unsafe PRNG usage patterns.
/// This contract intentionally contains vulnerabilities for testing purposes.
#[contract]
pub struct UnsafePrngExample;

#[contractimpl]
impl UnsafePrngExample {
    /// UNSAFE: Uses PRNG without reseeding in state-critical lottery draw.
    /// This function will be flagged by the unsafe_prng rule.
    pub fn draw_winner_unsafe(env: Env, participants: Vec<Address>) -> Address {
        let random_index: u64 = env.prng().gen_range(0..participants.len() as u64);
        let winner = participants.get(random_index as u32).unwrap();

        // Store winner in contract state - this makes it state-critical
        env.storage()
            .persistent()
            .set(&symbol_short!("winner"), &winner);

        winner
    }

    /// SAFE: Uses PRNG without storage mutation (read-only).
    /// This function will NOT be flagged.
    pub fn get_random_number(env: Env) -> u64 {
        env.prng().gen_range(0..100)
    }

    /// UNSAFE: Uses PRNG for token distribution with storage mutation.
    /// This function will be flagged by the unsafe_prng rule.
    pub fn distribute_rewards_unsafe(env: Env, recipients: Vec<Address>, amount: i128) {
        let random_bonus: u64 = env.prng().gen_range(1..10);

        for recipient in recipients.iter() {
            let final_amount = if random_bonus > 5 { amount * 2 } else { amount };

            // State mutation based on random value
            env.storage().persistent().set(&recipient, &final_amount);
        }
    }

    /// SAFE: Non-critical storage operation (no PRNG involved).
    /// This function will NOT be flagged.
    pub fn set_value(env: Env, key: u32, value: u64) {
        env.storage().persistent().set(&key, &value);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env};

    #[test]
    fn test_unsafe_draw_winner() {
        let env = Env::default();
        let contract_id = env.register_contract(None, UnsafePrngExample);
        let client = UnsafePrngExampleClient::new(&env, &contract_id);

        let participants = vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];

        let winner = client.draw_winner_unsafe(&participants);
        assert!(participants.contains(&winner));
    }

    #[test]
    fn test_get_random_number() {
        let env = Env::default();
        let contract_id = env.register_contract(None, UnsafePrngExample);
        let client = UnsafePrngExampleClient::new(&env, &contract_id);

        let random = client.get_random_number();
        assert!(random < 100);
    }
}
