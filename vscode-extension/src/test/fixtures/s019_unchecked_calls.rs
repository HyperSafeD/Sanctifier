#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct UncheckedCallFixture;

#[contractimpl]
impl UncheckedCallFixture {
    pub fn unsafe_external_call(env: Env, target: Address) {
        // ❌ RISK: Result of external call is ignored.
        // If the call fails, state remains inconsistent.
        let _ = env.invoke_contract::<()>(
            &target,
            &symbol_short!("ping"),
            soroban_sdk::vec![&env],
        );
    }
}
