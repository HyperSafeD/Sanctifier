use soroban_sdk::{contractimpl, Env};

struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn divide(env: Env, a: i128, b: i128) -> i128 {
        if b == 0 {
            panic!("division by zero");
        }
        a / b
    }

    pub fn unwrap_val(env: Env) -> i128 {
        let val: Option<i128> = None;
        val.unwrap()
    }
}
