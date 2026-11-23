//! Price reader that calls an oracle through `invoke_contract`, so a failing
//! oracle aborts the whole call instead of returning a handleable error.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, Env, Symbol};

const ORACLE: Symbol = symbol_short!("ORACLE");
const ADMIN: Symbol = symbol_short!("ADMIN");

#[contract]
pub struct OracleReader;

#[contractimpl]
impl OracleReader {
    pub fn read_price(env: Env) -> i128 {
        let admin: Address = match env.storage().instance().get(&ADMIN) {
            Some(addr) => addr,
            None => return 0,
        };
        admin.require_auth();
        let oracle: Address = match env.storage().instance().get(&ORACLE) {
            Some(addr) => addr,
            None => return 0,
        };
        env.invoke_contract::<i128>(&oracle, &symbol_short!("price"), vec![&env])
    }

    /// Handles the callee failure explicitly; must not be reported.
    pub fn read_price_checked(env: Env) -> i128 {
        let admin: Address = match env.storage().instance().get(&ADMIN) {
            Some(addr) => addr,
            None => return 0,
        };
        admin.require_auth();
        let oracle: Address = match env.storage().instance().get(&ORACLE) {
            Some(addr) => addr,
            None => return 0,
        };
        match env.try_invoke_contract::<i128, soroban_sdk::Error>(
            &oracle,
            &symbol_short!("price"),
            vec![&env],
        ) {
            Ok(Ok(price)) => price,
            _ => 0,
        }
    }
}
