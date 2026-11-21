//! Fixture for the `ledger_size` rule: a `#[contracttype]` record that embeds
//! a very large fixed-size history array, so a single ledger entry is
//! estimated above the 64 kB limit. Nothing else in the contract is unsafe, so
//! `ledger_size` is the only rule that should fire.
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env};

#[contracttype]
pub struct PriceArchive {
    pub feed: Address,
    pub latest: i128,
    pub history: [u64; 9000],
}

#[contract]
pub struct Archive;

#[contractimpl]
impl Archive {
    pub fn record(env: Env, feed: Address, latest: i128) {
        feed.require_auth();
        let key = symbol_short!("archive");
        let entry = PriceArchive {
            feed: feed.clone(),
            latest,
            history: [0; 9000],
        };
        env.storage().persistent().set(&key, &entry);
        env.storage().persistent().extend_ttl(&key, 100, 200);
        env.events().publish((symbol_short!("record"), feed), latest);
    }
}
