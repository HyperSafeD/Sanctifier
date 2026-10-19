#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, contracterror};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TokenError {
    Overflow = 1,
}

#[contract]
pub struct TokenWithBugs;

// Storage key for per-account balances.
const BALANCE: Symbol = symbol_short!("BALANCE");
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

#[contractimpl]
impl TokenWithBugs {
    pub fn initialize(e: Env, _admin: Address, _name: String, _symbol: String) {
    /// Initialise the token.
    ///
    /// NOTE – intentionally incomplete: does not persist `name`, or
    /// `symbol` so that Sanctifier can flag the missing initialisation guard.
    pub fn initialize(e: Env, admin: Address, _name: String, _symbol: String) {
        // Mark as initialised so re-entrancy can be detected.
        e.storage().instance().set(&symbol_short!("init"), &true);
        e.storage().instance().set(&ADMIN_KEY, &admin);
    }

    pub fn balance(e: Env, id: Address) -> i128 {
        e.storage().persistent().get(&id).unwrap_or(0)
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        // Assume from.require_auth() is added for S001 (if #1646 requires it, although not assigned, it's good)
        from.require_auth();
        let from_balance = Self::balance(e.clone(), from.clone());
        let new_from = from_balance.checked_sub(amount).ok_or(TokenError::Overflow)?;
        e.storage().persistent().set(&from, &new_from);

        let to_balance = Self::balance(e.clone(), to.clone());
        let new_to = to_balance.checked_add(amount).ok_or(TokenError::Overflow)?;
        e.storage().persistent().set(&to, &new_to);
        Ok(())
    }

    pub fn mint(e: Env, to: Address, amount: i128) -> Result<(), TokenError> {
    // VULNERABILITY: No overflow check – `current_balance + amount` can wrap.
    pub fn mint(e: Env, admin: Address, to: Address, amount: i128) {
        admin.require_auth();
        let stored_admin: Address = e.storage().instance().get(&ADMIN_KEY).unwrap();
        assert!(admin == stored_admin, "not admin");

        let current_balance = Self::balance(e.clone(), to.clone());
        let new_balance = current_balance.checked_add(amount).ok_or(TokenError::Overflow)?;
        e.storage().persistent().set(&to, &new_balance);
        Ok(())
    }

    pub fn transfer_from(e: Env, _spender: Address, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        let from_balance = Self::balance(e.clone(), from.clone());
        let new_from = from_balance.checked_sub(amount).ok_or(TokenError::Overflow)?;
        e.storage().persistent().set(&from, &new_from);

        let to_balance = Self::balance(e.clone(), to.clone());
        let new_to = to_balance.checked_add(amount).ok_or(TokenError::Overflow)?;
        e.storage().persistent().set(&to, &new_to);
        Ok(())
    }

    pub fn burn(e: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        from.require_auth();
        let current_balance = Self::balance(e.clone(), from.clone());
        let new_balance = current_balance.checked_sub(amount).ok_or(TokenError::Overflow)?;
        e.storage().persistent().set(&from, &new_balance);
        Ok(())
    }

    pub fn symbol(e: Env) -> String {
        let _ = BALANCE;
        String::from_str(&e, "TKN")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_burn_without_auth_fails() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TokenWithBugs);
        let client = TokenWithBugsClient::new(&env, &contract_id);
        
        let user1 = Address::generate(&env);
        // Mint to user1
        env.mock_all_auths();
        client.mint(&user1, &1000);
        
        // Remove mock auth
        env.mock_auths(&[]);
        
        // Try to burn without auth
        let res = client.try_burn(&user1, &500);
        assert!(res.is_err());
    }

    #[test]
    fn test_mint_overflow_returns_error() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TokenWithBugs);
        let client = TokenWithBugsClient::new(&env, &contract_id);
        
        let user = Address::generate(&env);
        
        // Mint max i128
        let max_val = i128::MAX;
        env.mock_all_auths();
        client.mint(&user, &max_val);
        
        // Try to mint 1 more, should fail with overflow
        let res = client.try_mint(&user, &1);
        assert_eq!(res, Err(Ok(TokenError::Overflow)));
    }
}
mod test;
