//! NullifierSet — persistent spent-nullifier registry with explicit TTL bumps.
//!
//! # TTL / Eviction Rationale
//! Soroban `persistent()` storage entries can be archived/evicted after
//! `max_ttl` ledgers if their TTL is not extended.  A silently-evicted
//! nullifier entry would read as "absent" rather than "spent", reintroducing
//! the double-spend risk that Z001 is meant to prevent.
//!
//! This module therefore:
//! 1. Bumps the TTL on every write (`mark_spent`) so spent entries persist;
//!    eviction after `max_ttl` is therefore treated as an operational risk,
//!    bounded by `TTL_BUMP_TO` (~1 year), not silently relied upon.
//! 2. **Rejects only what is provably spent.** A nullifier that is absent
//!    from storage is either genuinely unspent or — in the extreme —
//!    evicted after TTL expiry; both cases are indistinguishable, and the
//!    first-time spend must be allowed.  The absence of an entry is therefore
//!    treated as *unspent*, never as a double-spend.

use soroban_sdk::{contracttype, Bytes, Env};

/// Minimum number of ledgers a nullifier entry must survive after being
/// written. 1 year ≈ 6 307 200 ledgers at 5 s/ledger.
const TTL_BUMP_THRESHOLD: u32 = 100_000;
/// Target ledger count for TTL extension — significantly above threshold so
/// we do not bump on every single transaction.
const TTL_BUMP_TO: u32 = 6_307_200; // ~1 year

#[derive(Clone, PartialEq)]
#[contracttype]
pub enum NullifierState {
    /// Nullifier has been spent; the associated proof must not be accepted again.
    Spent,
}

/// Composite storage key scoping nullifiers to a specific campaign or context.
#[contracttype]
#[derive(Clone)]
pub struct NullifierKey {
    pub context: Bytes,
    pub nullifier: Bytes,
}

/// Thin wrapper around Soroban persistent storage providing a
/// spend-check / mark-spent API with explicit TTL management.
pub struct NullifierSet;

impl Default for NullifierSet {
    fn default() -> Self {
        Self::new()
    }
}

impl NullifierSet {
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` if `nullifier` is recorded as spent.
    ///
    /// Returns `false` for both the "unspent" and the "evicted / absent"
    /// cases; the two are indistinguishable at storage level.
    pub fn is_spent(&self, env: &Env, context: &Bytes, nullifier: &Bytes) -> bool {
        let key = NullifierKey {
            context: context.clone(),
            nullifier: nullifier.clone(),
        };
        env.storage()
            .persistent()
            .get::<NullifierKey, NullifierState>(&key)
            .map(|s| s == NullifierState::Spent)
            .unwrap_or(false)
    }

    /// Returns the raw `Option<NullifierState>` from storage.
    ///
    /// `None` means the entry is absent — either genuinely unspent *or*
    /// evicted due to TTL expiry; the two cannot be distinguished here.
    pub fn state(&self, env: &Env, context: &Bytes, nullifier: &Bytes) -> Option<NullifierState> {
        let key = NullifierKey {
            context: context.clone(),
            nullifier: nullifier.clone(),
        };
        env.storage()
            .persistent()
            .get::<NullifierKey, NullifierState>(&key)
    }

    /// Panics if `nullifier` is provably spent.
    ///
    /// A nullifier that has never been written is allowed to spend; the call
    /// site must then `mark_spent` before returning to close the TOCTOU
    /// window.  Entries evicted after `TTL_BUMP_TO` read as absent and are
    /// therefore treated as unspent; keep spent entries' TTL bumped so this
    /// residual window stays bounded by the bump interval.
    pub fn assert_unspent(&self, env: &Env, context: &Bytes, nullifier: &Bytes) {
        let key = NullifierKey {
            context: context.clone(),
            nullifier: nullifier.clone(),
        };
        match env
            .storage()
            .persistent()
            .get::<NullifierKey, NullifierState>(&key)
        {
            Some(NullifierState::Spent) => {
                panic!("nullifier already spent — replay attack rejected")
            }
            None => {
                // Absent = never spent (or evicted well past the TTL bump
                // window); first-time spends are legitimate.  Allow and let
                // the caller `mark_spent` immediately after.
            }
        }
    }

    /// Mark `nullifier` as spent and bump its TTL.
    ///
    /// Must be called **after** `assert_unspent` and **before** returning
    /// from the verify function to prevent TOCTOU races in parallel
    /// transaction scenarios.
    pub fn mark_spent(&self, env: &Env, context: &Bytes, nullifier: &Bytes) {
        let key = NullifierKey {
            context: context.clone(),
            nullifier: nullifier.clone(),
        };
        env.storage()
            .persistent()
            .set::<NullifierKey, NullifierState>(&key, &NullifierState::Spent);
        // Explicit TTL bump — without this the entry expires after max_ttl
        // ledgers and an evicted nullifier would no longer protect against
        // replays.
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_BUMP_THRESHOLD, TTL_BUMP_TO);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ZkVerifier;
    use soroban_sdk::{Bytes, Env};

    fn ctx(env: &Env) -> Bytes {
        Bytes::from_slice(env, b"test-campaign")
    }

    fn null(env: &Env, n: u8) -> Bytes {
        Bytes::from_slice(env, &[n; 32])
    }

    /// Storage operations require an active contract execution frame, so
    /// unit tests exercise `NullifierSet` inside `Env::as_contract`.
    fn with_contract<F: FnOnce(&Env)>(f: F) {
        let env = Env::default();
        let contract_id = env.register_contract(None, ZkVerifier);
        env.as_contract(&contract_id, || f(&env));
    }

    #[test]
    fn unspent_nullifier_is_not_spent() {
        with_contract(|env| {
            let ns = NullifierSet::new();
            assert!(!ns.is_spent(env, &ctx(env), &null(env, 0x01)));
        });
    }

    #[test]
    fn mark_spent_records_nullifier() {
        with_contract(|env| {
            let ns = NullifierSet::new();
            let nullifier = null(env, 0x02);
            ns.mark_spent(env, &ctx(env), &nullifier);
            assert!(ns.is_spent(env, &ctx(env), &nullifier));
        });
    }

    #[test]
    #[should_panic(expected = "nullifier already spent")]
    fn assert_unspent_panics_on_spent() {
        with_contract(|env| {
            let ns = NullifierSet::new();
            let nullifier = null(env, 0x03);
            ns.mark_spent(env, &ctx(env), &nullifier);
            ns.assert_unspent(env, &ctx(env), &nullifier);
        });
    }

    #[test]
    fn absent_nullifier_is_allowed_once() {
        with_contract(|env| {
            let ns = NullifierSet::new();
            // Absent (never spent / evicted) is indistinguishable from a
            // legitimate first-time spend, so it must be allowed; the test
            // passes only if this does not panic.
            let nullifier = null(env, 0x04);
            ns.assert_unspent(env, &ctx(env), &nullifier);
        });
    }

    #[test]
    #[should_panic(expected = "nullifier already spent")]
    fn spent_nullifier_is_rejected() {
        with_contract(|env| {
            let ns = NullifierSet::new();
            let nullifier = null(env, 0x05);
            ns.mark_spent(env, &ctx(env), &nullifier);
            ns.assert_unspent(env, &ctx(env), &nullifier);
        });
    }
}
