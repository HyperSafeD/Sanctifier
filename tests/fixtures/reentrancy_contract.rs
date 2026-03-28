#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct ReentrancyContract;

#[contractimpl]
impl ReentrancyContract {
    pub fn execute(env: Env, target: Address, amount: i128) {
        let fn_name = Symbol::new(&env, "callback");
        env.invoke_contract::<()>(&target, &fn_name, (&amount,));
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "last_amount"), &amount);
    }
}
