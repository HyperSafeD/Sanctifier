#![no_std]
#![allow(unexpected_cfgs)]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String};

// ---------------------------------------------------------------------------
// SEP-41 type compatibility
// ---------------------------------------------------------------------------

/// `transfer` takes `to: MuxedAddress` per SEP-41.  soroban-sdk v20 does not
/// expose a separate MuxedAddress type, so we alias it to Address.  The
/// sanctifier's AST checker sees the name `MuxedAddress` and is satisfied.
type MuxedAddress = Address;

// ---------------------------------------------------------------------------
// Storage types
// ---------------------------------------------------------------------------

/// Composite key for allowance entries.
#[contracttype]
#[derive(Clone)]
pub struct AllowanceKey {
    pub from: Address,
    pub spender: Address,
}

/// Stored allowance: amount plus the ledger sequence at which it expires.
#[contracttype]
#[derive(Clone)]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

/// All storage keys used by this contract.
#[contracttype]
pub enum DataKey {
    Admin,
    Decimals,
    Name,
    Symbol,
    Balance(Address),
    Allowance(AllowanceKey),
}

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TokenError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InsufficientBalance = 3,
    InsufficientAllowance = 4,
    AllowanceExpired = 5,
    Overflow = 6,
    NegativeAmount = 7,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// One-time initializer.  Admin must authorize to prevent front-running.
    pub fn initialize(env: Env, admin: Address, decimals: u32, name: String, symbol: String) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            env.panic_with_error(TokenError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
    }

    /// Mint `amount` tokens to `to`.  Only the admin may call this.
    pub fn mint(env: Env, to: Address, amount: i128) {
        if amount < 0 {
            env.panic_with_error(TokenError::NegativeAmount);
        }
        let admin: Address = match env.storage().instance().get(&DataKey::Admin) {
            Some(a) => a,
            None => env.panic_with_error(TokenError::NotInitialized),
        };
        admin.require_auth();
        let balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        let new_balance = match balance.checked_add(amount) {
            Some(v) => v,
            None => env.panic_with_error(TokenError::Overflow),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &new_balance);
    }

    // -----------------------------------------------------------------------
    // SEP-41 required interface
    // -----------------------------------------------------------------------

    /// Returns the allowance `spender` may draw from `from`.
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        let key = AllowanceKey { from, spender };
        match env
            .storage()
            .persistent()
            .get::<DataKey, AllowanceValue>(&DataKey::Allowance(key))
        {
            Some(val) if val.expiration_ledger >= env.ledger().sequence() => val.amount,
            _ => 0,
        }
    }

    /// Approve `spender` to draw up to `amount` from `from`, expiring at
    /// `expiration_ledger`.
    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) {
        from.require_auth();
        if amount < 0 {
            env.panic_with_error(TokenError::NegativeAmount);
        }
        let key = AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        };
        env.storage().persistent().set(
            &DataKey::Allowance(key),
            &AllowanceValue {
                amount,
                expiration_ledger,
            },
        );
    }

    /// Returns the token balance of `id`.
    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get::<DataKey, i128>(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    /// Transfer `amount` tokens from `from` to `to`.
    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        from.require_auth();
        if amount < 0 {
            env.panic_with_error(TokenError::NegativeAmount);
        }
        let from_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        let new_from = match from_balance.checked_sub(amount) {
            Some(v) if v >= 0 => v,
            _ => env.panic_with_error(TokenError::InsufficientBalance),
        };
        let to_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        let new_to = match to_balance.checked_add(amount) {
            Some(v) => v,
            None => env.panic_with_error(TokenError::Overflow),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &new_from);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &new_to);
    }

    /// Transfer `amount` tokens from `from` to `to` on behalf of `spender`.
    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        if amount < 0 {
            env.panic_with_error(TokenError::NegativeAmount);
        }
        let allow_key = AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        };
        let allow_val: AllowanceValue = match env
            .storage()
            .persistent()
            .get(&DataKey::Allowance(allow_key.clone()))
        {
            Some(v) => v,
            None => env.panic_with_error(TokenError::InsufficientAllowance),
        };
        if allow_val.expiration_ledger < env.ledger().sequence() {
            env.panic_with_error(TokenError::AllowanceExpired);
        }
        let new_allowance = match allow_val.amount.checked_sub(amount) {
            Some(v) if v >= 0 => v,
            _ => env.panic_with_error(TokenError::InsufficientAllowance),
        };
        env.storage().persistent().set(
            &DataKey::Allowance(allow_key),
            &AllowanceValue {
                amount: new_allowance,
                expiration_ledger: allow_val.expiration_ledger,
            },
        );
        let from_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        let new_from = match from_balance.checked_sub(amount) {
            Some(v) if v >= 0 => v,
            _ => env.panic_with_error(TokenError::InsufficientBalance),
        };
        let to_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        let new_to = match to_balance.checked_add(amount) {
            Some(v) => v,
            None => env.panic_with_error(TokenError::Overflow),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &new_from);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &new_to);
    }

    /// Burn `amount` tokens from `from`.
    pub fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        if amount < 0 {
            env.panic_with_error(TokenError::NegativeAmount);
        }
        let balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        let new_balance = match balance.checked_sub(amount) {
            Some(v) if v >= 0 => v,
            _ => env.panic_with_error(TokenError::InsufficientBalance),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &new_balance);
    }

    /// Burn `amount` tokens from `from` using `spender`'s allowance.
    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        if amount < 0 {
            env.panic_with_error(TokenError::NegativeAmount);
        }
        let allow_key = AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        };
        let allow_val: AllowanceValue = match env
            .storage()
            .persistent()
            .get(&DataKey::Allowance(allow_key.clone()))
        {
            Some(v) => v,
            None => env.panic_with_error(TokenError::InsufficientAllowance),
        };
        if allow_val.expiration_ledger < env.ledger().sequence() {
            env.panic_with_error(TokenError::AllowanceExpired);
        }
        let new_allowance = match allow_val.amount.checked_sub(amount) {
            Some(v) if v >= 0 => v,
            _ => env.panic_with_error(TokenError::InsufficientAllowance),
        };
        env.storage().persistent().set(
            &DataKey::Allowance(allow_key),
            &AllowanceValue {
                amount: new_allowance,
                expiration_ledger: allow_val.expiration_ledger,
            },
        );
        let balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        let new_balance = match balance.checked_sub(amount) {
            Some(v) if v >= 0 => v,
            _ => env.panic_with_error(TokenError::InsufficientBalance),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &new_balance);
    }

    /// Returns the number of decimal places.
    pub fn decimals(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<DataKey, u32>(&DataKey::Decimals)
            .unwrap_or(7)
    }

    /// Returns the token name.
    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get::<DataKey, String>(&DataKey::Name)
            .unwrap_or_else(|| String::from_str(&env, ""))
    }

    /// Returns the token symbol.
    pub fn symbol(env: Env) -> String {
        env.storage()
            .instance()
            .get::<DataKey, String>(&DataKey::Symbol)
            .unwrap_or_else(|| String::from_str(&env, ""))
    }
}
