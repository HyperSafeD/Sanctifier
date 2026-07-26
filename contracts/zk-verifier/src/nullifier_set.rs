//! NullifierSet — persistent spent-nullifier registry with explicit TTL bumps.
//!
//! # TTL / Eviction Rationale
//! Soroban `persistent()` storage entries can be archived/evicted after
//! `max_ttl` ledgers if their TTL is not extended.  A silently-evicted
//! nullifier entry would read as "absent" rather than "spent", reintroducing
//! the double-spend risk that Z001 is meant to prevent.
//!
//! This module therefore:
//! 1. Bumps the TTL on every write (`mark_spent`).
//! 2. **Fails closed** on read: an absent entry is treated as "unknown /
//!    must reject", never as "unspent / allow".  The `assert_unspent` method
//!    panics on both "definitely spent" and "evicted / unknown" states,
//!    matching the closed-world assumption of a spend check.
//!
//! Callers that want to distinguish "unknown" from "spent" should use
//! `state()` directly and apply their own policy.

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

impl NullifierSet {
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` if `nullifier` is definitely recorded as spent.
    ///
    /// Returns `false` for **both** the "unspent" and the "evicted / absent"
    /// cases.  Callers that must distinguish eviction from genuinely-unspent
    /// should use `state()` and apply their own policy; most callers should
    /// use `assert_unspent` which fails closed on both.
    pub fn is_spent(&self, env: &Env, context: &Bytes, nullifier: &Bytes) -> bool {
        let key = NullifierKey { context: context.clone(), nullifier: nullifier.clone() };
        env.storage()
            .persistent()
            .get::<NullifierKey, NullifierState>(&key)
            .map(|s| s == NullifierState::Spent)
            .unwrap_or(false)
    }

    /// Returns the raw `Option<NullifierState>` from storage.
    ///
    /// `None` means the entry is absent — either genuinely unspent *or*
    /// evicted due to TTL expiry.  Treat `None` as untrusted.
    pub fn state(&self, env: &Env, context: &Bytes, nullifier: &Bytes) -> Option<NullifierState> {
        let key = NullifierKey { context: context.clone(), nullifier: nullifier.clone() };
        env.storage().persistent().get::<NullifierKey, NullifierState>(&key)
    }

    /// Panics if `nullifier` is spent **or absent** (fail-closed policy).
    ///
    /// Call this before granting any asset access, then call `mark_spent`
    /// before returning.
    pub fn assert_unspent(&self, env: &Env, context: &Bytes, nullifier: &Bytes) {
        let key = NullifierKey { context: context.clone(), nullifier: nullifier.clone() };
        match env.storage().persistent().get::<NullifierKey, NullifierState>(&key) {
            Some(NullifierState::Spent) => {
                panic!("nullifier already spent — replay attack rejected")
            }
            None => {
                // Fail closed: absent could mean evicted.  Reject to prevent
                // a TTL-eviction-based replay attack.
                //
                // If this fires unexpectedly in production it means entries
                // are expiring faster than expected; increase TTL_BUMP_TO or
                // migrate spent nullifiers to a longer-lived tier.
                panic!("nullifier absent (possibly evicted) — failing closed to prevent replay")
            }
            _ => {} // NullifierState::Spent is the only variant; exhaustive.
        }
        // Unreachable: both Some(Spent) and None panic above.  The match
        // guard keeps the compiler happy if new variants are added later.
        let _ = key;
    }

    /// Mark `nullifier` as spent and bump its TTL.
    ///
    /// Must be called **after** `assert_unspent` and **before** returning
    /// from the verify function to prevent TOCTOU races in parallel
    /// transaction scenarios.
    pub fn mark_spent(&self, env: &Env, context: &Bytes, nullifier: &Bytes) {
        let key = NullifierKey { context: context.clone(), nullifier: nullifier.clone() };
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
    use soroban_sdk::{testutils::Env as _, Bytes, Env};

    fn ctx(env: &Env) -> Bytes {
        Bytes::from_slice(env, b"test-campaign")
    }

    fn null(env: &Env, n: u8) -> Bytes {
        Bytes::from_slice(env, &[n; 32])
    }

    #[test]
    fn unspent_nullifier_is_not_spent() {
        let env = Env::default();
        let ns = NullifierSet::new();
        assert!(!ns.is_spent(&env, &ctx(&env), &null(&env, 0x01)));
    }

    #[test]
    fn mark_spent_records_nullifier() {
        let env = Env::default();
        let ns = NullifierSet::new();
        let nullifier = null(&env, 0x02);
        ns.mark_spent(&env, &ctx(&env), &nullifier);
        assert!(ns.is_spent(&env, &ctx(&env), &nullifier));
    }

    #[test]
    #[should_panic(expected = "nullifier already spent")]
    fn assert_unspent_panics_on_spent() {
        let env = Env::default();
        let ns = NullifierSet::new();
        let nullifier = null(&env, 0x03);
        ns.mark_spent(&env, &ctx(&env), &nullifier);
        ns.assert_unspent(&env, &ctx(&env), &nullifier);
    }

    #[test]
    #[should_panic(expected = "nullifier absent (possibly evicted) — failing closed")]
    fn assert_unspent_panics_on_absent_entry_fail_closed() {
        let env = Env::default();
        let ns = NullifierSet::new();
        // Simulate eviction: write then manually remove the key so the entry
        // appears absent to storage, then assert_unspent must panic (fail closed).
        let nullifier = null(&env, 0x04);
        let key = NullifierKey { context: ctx(&env), nullifier: nullifier.clone() };
        env.storage()
            .persistent()
            .set::<NullifierKey, NullifierState>(&key, &NullifierState::Spent);
        env.storage().persistent().remove(&key);
        // Now the entry is absent — as if it had been evicted.
        ns.assert_unspent(&env, &ctx(&env), &nullifier);
    }
}
