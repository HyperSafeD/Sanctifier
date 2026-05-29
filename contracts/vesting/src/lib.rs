#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum VestingError {
    AlreadyInitialized = 1,
    NotRevocable = 2,
    AlreadyRevoked = 3,
    NoVestedTokens = 4,
}

#[contract]
pub struct VestingContract;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Beneficiary,
    Token,
    Start,
    Cliff,
    Duration,
    TotalAmount,
    Released,
    Revocable,
    RevokedAt,
}

#[contractimpl]
impl VestingContract {
    /// Create a new vesting schedule.
    pub fn init(
        env: Env,
        admin: Address,
        beneficiary: Address,
        token: Address,
        start: u64,
        cliff: u64,
        duration: u64,
        amount: i128,
        revocable: bool,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            env.panic_with_error(VestingError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Beneficiary, &beneficiary);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Start, &start);
        env.storage().instance().set(&DataKey::Cliff, &cliff);
        env.storage().instance().set(&DataKey::Duration, &duration);
        env.storage().instance().set(&DataKey::TotalAmount, &amount);
        env.storage().instance().set(&DataKey::Released, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::Revocable, &revocable);
    }

    /// Returns the total amount vested based on current time.
    pub fn vested_amount(env: Env) -> i128 {
        let start: u64 = env.storage().instance().get(&DataKey::Start).unwrap();
        let cliff: u64 = env.storage().instance().get(&DataKey::Cliff).unwrap();
        let duration: u64 = env.storage().instance().get(&DataKey::Duration).unwrap();
        let total_amount: i128 = env.storage().instance().get(&DataKey::TotalAmount).unwrap();
        let revoked_at: Option<u64> = env.storage().instance().get(&DataKey::RevokedAt);

        let current_time = if let Some(revoked_at) = revoked_at {
            revoked_at
        } else {
            env.ledger().timestamp()
        };

        if current_time < start + cliff {
            return 0;
        }

        if current_time >= start + duration {
            return total_amount;
        }

        total_amount * (current_time - start) as i128 / duration as i128
    }

    /// Returns the amount currently available to be claimed.
    pub fn claimable_amount(env: Env) -> i128 {
        let vested = Self::vested_amount(env.clone());
        let released: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Released)
            .unwrap_or(0);
        vested - released
    }

    /// Claim tokens from vesting schedule.
    pub fn claim(env: Env) -> i128 {
        let beneficiary: Address = env.storage().instance().get(&DataKey::Beneficiary).unwrap();
        beneficiary.require_auth();

        let claimable = Self::claimable_amount(env.clone());
        if claimable <= 0 {
            env.panic_with_error(VestingError::NoVestedTokens);
        }

        let token_id: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let released: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Released)
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&DataKey::Released, &(released + claimable));

        let token_client = token::TokenClient::new(&env, &token_id);
        token_client.transfer(&env.current_contract_address(), &beneficiary, &claimable);

        claimable
    }

    /// Revoke the vesting schedule.
    pub fn revoke(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let revocable: bool = env.storage().instance().get(&DataKey::Revocable).unwrap();
        if !revocable {
            env.panic_with_error(VestingError::NotRevocable);
        }

        if env.storage().instance().has(&DataKey::RevokedAt) {
            env.panic_with_error(VestingError::AlreadyRevoked);
        }

        let current_time = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::RevokedAt, &current_time);

        let token_id: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let total_amount: i128 = env.storage().instance().get(&DataKey::TotalAmount).unwrap();
        let vested = Self::vested_amount(env.clone());

        let unvested = total_amount - vested;

        if unvested > 0 {
            let token_client = token::TokenClient::new(&env, &token_id);
            token_client.transfer(&env.current_contract_address(), &admin, &unvested);
        }
    }
}
