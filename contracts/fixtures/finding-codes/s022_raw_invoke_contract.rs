#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct RawInvokeFixture;

#[contractimpl]
impl RawInvokeFixture {
    /// ❌ S022: `invoke_contract` panics when the callee returns an error.
    /// There is no way for the caller to recover — the transaction aborts.
    pub fn unsafe_cross_call(env: Env, target: Address) {
        env.invoke_contract::<()>(
            &target,
            &symbol_short!("ping"),
            soroban_sdk::vec![&env],
        );
    }

    /// ✅ Preferred: `try_invoke_contract` surfaces the callee result as a
    /// typed `Result`, allowing the caller to handle failure without panicking.
    pub fn safe_cross_call(env: Env, target: Address) -> Result<(), soroban_sdk::Error> {
        env.try_invoke_contract::<(), soroban_sdk::Error>(
            &target,
            &symbol_short!("ping"),
            soroban_sdk::vec![&env],
        )?;
        Ok(())
    }
}
