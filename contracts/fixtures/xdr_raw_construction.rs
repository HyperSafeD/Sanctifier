#![no_std]
use soroban_sdk::{contractimpl, Env, Symbol};

pub struct XdrRawConstructionContract;

#[contractimpl]
impl XdrRawConstructionContract {
    pub fn store_raw_value(env: Env) {
        let raw_value = xdr::ScVal::U64(42);
        let raw_vector = xdr::ScVal::Vec(vec![xdr::ScVal::Symbol(Symbol::new(&env, "demo"))]);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "raw"), &raw_value);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "raw_vector"), &raw_vector);
    }
}
