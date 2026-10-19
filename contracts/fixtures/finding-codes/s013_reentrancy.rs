#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct ReentrancyFixture;

#[contractimpl]
impl ReentrancyFixture {
    pub fn vulnerable_transfer(env: Env, to: Address, amount: i128) {
        // ❌ VULNERABLE: State mutation happens AFTER external call.
        // If `to` is a contract that calls back into this function,
        // it can drain the balance.
        env.invoke_contract::<()>(
            &to,
            &symbol_short!("receive"),
            soroban_sdk::vec![&env, amount.into_val(&env)],
        );

        let balance: i128 = env.storage().persistent().get(&symbol_short!("BAL")).unwrap_or(0);
        env.storage().persistent().set(&symbol_short!("BAL"), &(balance - amount));
    }
}
