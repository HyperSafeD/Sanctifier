use soroban_sdk::{contractimpl, Env};

struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn transfer(env: Env) {
        env.storage().persistent().set(&"balance", &100i128);
    }
}
