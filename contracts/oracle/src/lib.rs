#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InvalidThreshold = 3,
    InsufficientSigners = 4,
    StalePrice = 5,
}

#[contracttype]
pub enum DataKey {
    Validators,
    Threshold,
    PriceData,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PriceData {
    pub price: u128,
    pub timestamp: u64,
}

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize the oracle with a set of trusted validators and a consensus threshold.
    pub fn init(env: Env, validators: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Threshold) {
            env.panic_with_error(Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold > validators.len() {
            env.panic_with_error(Error::InvalidThreshold);
        }
        env.storage()
            .instance()
            .set(&DataKey::Validators, &validators);
        env.storage()
            .instance()
            .set(&DataKey::Threshold, &threshold);
    }

    /// Update the price feed. Requires multi-sig authorization from validators.
    pub fn update_price(env: Env, price: u128, timestamp: u64, validators_approving: Vec<Address>) {
        let trusted_validators: Vec<Address> =
            env.storage().instance().get(&DataKey::Validators).unwrap();
        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();

        let mut valid_count = 0;
        let mut processed = Vec::<Address>::new(&env);

        for validator in validators_approving {
            if trusted_validators.contains(&validator) && !processed.contains(&validator) {
                validator.require_auth();
                valid_count += 1;
                processed.push_back(validator);
            }
        }

        if valid_count < threshold {
            env.panic_with_error(Error::InsufficientSigners);
        }

        env.storage()
            .instance()
            .set(&DataKey::PriceData, &PriceData { price, timestamp });

        env.events()
            .publish((symbol_short!("price_upd"),), (price, timestamp));
    }

    /// Read the latest price. Validates that the price is not older than max_age.
    pub fn get_price(env: Env, max_age: u64) -> u128 {
        let data: PriceData = env
            .storage()
            .instance()
            .get(&DataKey::PriceData)
            .unwrap_or_else(|| {
                env.panic_with_error(Error::StalePrice);
            });

        let current_time = env.ledger().timestamp();
        if current_time > data.timestamp + max_age {
            env.panic_with_error(Error::StalePrice);
        }

        data.price
    }
}
