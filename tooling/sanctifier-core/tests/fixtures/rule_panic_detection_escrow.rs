//! Escrow contract whose entry points abort through `panic!`, `unwrap` and
//! `expect`. The `#[cfg(test)]` module below must be ignored by the rule.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

const RELEASED: Symbol = symbol_short!("RELEASED");

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    pub fn release(env: Env, beneficiary: Address, amount: i128) {
        beneficiary.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }
        env.storage().persistent().set(&RELEASED, &amount);
        env.storage().persistent().extend_ttl(&RELEASED, 17_280, 17_280);
        env.events().publish((symbol_short!("released"),), amount);
    }

    pub fn released(env: Env) -> i128 {
        env.storage().persistent().get(&RELEASED).unwrap()
    }

    pub fn beneficiary_of(env: Env, key: Symbol) -> Address {
        env.storage().persistent().get(&key).expect("missing beneficiary")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn released_is_zero_before_release() {
        let env = Env::default();
        let id = env.register_contract(None, Escrow);
        let value: Option<i128> = env.as_contract(&id, || env.storage().persistent().get(&RELEASED));
        assert_eq!(value.unwrap_or(0), 0);
        None::<i128>.unwrap();
    }
}
