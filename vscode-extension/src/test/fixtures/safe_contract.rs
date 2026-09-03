use soroban_sdk::{contractimpl, Address, Env};

struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn transfer(env: Env, from: Address) {
        from.require_auth();
        env.storage().persistent().set(&"balance", &100i128);
    }
}
