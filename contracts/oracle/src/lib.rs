#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Vec,
};

#[cfg(test)]
mod test;

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
    LastUpdated,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PriceData {
    pub price: u128,
    pub timestamp: u64,
}

/// Event emitted by heartbeat() to allow monitoring systems to track feeder health.
#[contracttype]
#[derive(Clone, Debug)]
pub struct HeartbeatEvent {
    pub last_update: u64,
    pub current_time: u64,
    pub time_since_update: u64,
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
        env.storage()
            .instance()
            .set(&DataKey::LastUpdated, &env.ledger().timestamp());

        env.events()
            .publish((symbol_short!("price_upd"),), (price, timestamp));
    }

    /// Read the latest price. Validates that the on-chain update is not older
    /// than `max_age` seconds.
    ///
    /// Staleness is measured from the ledger timestamp recorded by
    /// [`update_price`], not the feeder-supplied timestamp stored in
    /// [`PriceData`]. This prevents a feeder from making old data appear fresh.
    pub fn get_price(env: Env, max_age: u64) -> u128 {
        let data: PriceData = env
            .storage()
            .instance()
            .get(&DataKey::PriceData)
            .unwrap_or_else(|| {
                env.panic_with_error(Error::StalePrice);
            });

        let updated_at: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastUpdated)
            .unwrap_or_else(|| env.panic_with_error(Error::StalePrice));
        let age = env.ledger().timestamp().saturating_sub(updated_at);
        if age > max_age {
            env.panic_with_error(Error::StalePrice);
        }

        data.price
    }

    /// Heartbeat function for monitoring the oracle's health.
    /// Emits an event containing the last update timestamp and time elapsed since last update.
    /// 
    /// Monitoring systems should call this periodically to:
    /// - Track if the price feeder is still active
    /// - Alert when time_since_update exceeds acceptable thresholds
    /// - Verify the oracle is operational
    ///
    /// # Monitoring Integration
    /// 
    /// Example monitoring setup:
    /// 1. Call `heartbeat()` every 60 seconds
    /// 2. Listen for `heartbeat` events
    /// 3. Alert if `time_since_update` > expected feed interval
    /// 4. Alert if no `heartbeat` events received for N intervals
    ///
    /// # Returns
    /// Emits a `HeartbeatEvent` containing:
    /// - `last_update`: timestamp of the last price update
    /// - `current_time`: current ledger timestamp
    /// - `time_since_update`: seconds elapsed since last update
    pub fn heartbeat(env: Env) {
        let data: Option<PriceData> = env.storage().instance().get(&DataKey::PriceData);
        let current_time = env.ledger().timestamp();

        match data {
            Some(price_data) => {
                let time_since_update = current_time.saturating_sub(price_data.timestamp);
                env.events().publish(
                    (symbol_short!("heartbeat"),),
                    HeartbeatEvent {
                        last_update: price_data.timestamp,
                        current_time,
                        time_since_update,
                    },
                );
            }
            None => {
                // No price data yet - emit heartbeat with zero last_update
                env.events().publish(
                    (symbol_short!("heartbeat"),),
                    HeartbeatEvent {
                        last_update: 0,
                        current_time,
                        time_since_update: current_time,
                    },
                );
            }
        }
    }

    /// Check if the oracle is alive (has recent data).
    /// 
    /// Returns `true` if the oracle has price data that is newer than `max_age` seconds,
    /// `false` otherwise.
    ///
    /// This is a view function that allows contracts and monitoring systems to check
    /// feeder health without reverting.
    ///
    /// # Arguments
    /// - `max_age`: Maximum acceptable age in seconds for the last price update
    ///
    /// # Returns
    /// - `true` if oracle has data and it's fresh (age <= max_age)
    /// - `false` if oracle has no data or data is stale (age > max_age)
    ///
    /// # Example
    /// ```ignore
    /// // Check if oracle has data from the last hour (3600 seconds)
    /// if oracle.is_alive(3600) {
    ///     let price = oracle.get_price(3600);
    ///     // Use price...
    /// } else {
    ///     // Feeder is down, use fallback...
    /// }
    /// ```
    pub fn is_alive(env: Env, max_age: u64) -> bool {
        let data: Option<PriceData> = env.storage().instance().get(&DataKey::PriceData);

        match data {
            Some(price_data) => {
                let current_time = env.ledger().timestamp();
                let age = current_time.saturating_sub(price_data.timestamp);
                age <= max_age
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _};

    fn setup(env: &Env) -> (OracleContractClient<'_>, Address) {
        env.mock_all_auths();
        let validator = Address::generate(env);
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(env, &contract_id);
        client.init(&Vec::from_array(env, [validator.clone()]), &1);
        (client, validator)
    }

    #[test]
    fn stale_price_returns_error() {
        let env = Env::default();
        env.ledger().set_timestamp(1_000);
        let (client, validator) = setup(&env);

        // A feeder-supplied future timestamp must not make old data look fresh.
        client.update_price(&42, &u64::MAX, &Vec::from_array(&env, [validator]));
        env.ledger().set_timestamp(1_011);

        assert!(client.try_get_price(&10).is_err());
    }

    #[test]
    fn every_update_refreshes_last_updated_time() {
        let env = Env::default();
        env.ledger().set_timestamp(100);
        let (client, validator) = setup(&env);
        let validators = Vec::from_array(&env, [validator]);

        client.update_price(&10, &1, &validators);
        env.ledger().set_timestamp(109);
        client.update_price(&11, &1, &validators);
        env.ledger().set_timestamp(115);

        assert_eq!(client.get_price(&6), 11);
    }
}
