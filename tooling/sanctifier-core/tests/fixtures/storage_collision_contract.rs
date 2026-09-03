// Fixture for S005 storage key collision tests.
//
// Deliberately reuses the same persistent storage key "BALANCE" from
// two independent functions, which can silently overwrite each other's data.
// This is the canonical threat: two logical sub-systems share one storage
// namespace without coordination, causing phantom reads/writes.
use soroban_sdk::{contract, contractimpl, symbol_short, Env};

#[contract]
pub struct CollisionContract;

#[contractimpl]
impl CollisionContract {
    pub fn set_stake(env: Env, amount: i128) {
        env.storage()
            .persistent()
            .set(&symbol_short!("BALANCE"), &amount);
    }

    pub fn set_reward(env: Env, amount: i128) {
        env.storage()
            .persistent()
            .set(&symbol_short!("BALANCE"), &amount);
    }
}
