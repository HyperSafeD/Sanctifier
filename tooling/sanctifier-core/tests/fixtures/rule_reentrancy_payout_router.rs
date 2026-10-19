//! Payout router that settles a claim by calling a hook contract after it has
//! already updated state, with no reentrancy lock around the call.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, Env, IntoVal, Symbol};

const CLAIMED: Symbol = symbol_short!("CLAIMED");
const CLAIM_TTL: u32 = 17_280;

#[contract]
pub struct PayoutRouter;

#[contractimpl]
impl PayoutRouter {
    pub fn settle(env: Env, claimant: Address, hook: Address, amount: i128) -> i128 {
        claimant.require_auth_for_args((amount,).into_val(&env));
        env.storage().persistent().set(&CLAIMED, &amount);
        env.storage().persistent().extend_ttl(&CLAIMED, CLAIM_TTL, CLAIM_TTL);
        env.events().publish((symbol_short!("settled"),), amount);
        let acknowledged: i128 =
            env.invoke_contract(&hook, &symbol_short!("on_settle"), vec![&env]);
        acknowledged
    }
}
