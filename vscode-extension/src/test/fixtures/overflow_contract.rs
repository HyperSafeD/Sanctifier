use soroban_sdk::{contractimpl, Env};

struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn add(env: Env, a: i128, b: i128) -> i128 {
        a + b
    }

    pub fn safe_add(env: Env, a: i128, b: i128) -> Option<i128> {
        a.checked_add(b)
    }
}
