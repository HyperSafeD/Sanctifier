//! Pre-configured [`Env`] for contract tests.

use soroban_sdk::{testutils::Ledger, Env};

/// Minimum default ledger timestamp so expiry/TTL logic works in tests.
pub const DEFAULT_LEDGER_TIMESTAMP: u64 = 1_700_000_000;

/// Default sequence number written to the mock ledger.
pub const DEFAULT_SEQUENCE: u32 = 100;

/// A thin wrapper that holds a Soroban [`Env`] pre-configured for testing.
///
/// ```rust,ignore
/// use sanctifier_test_support::env::TestEnv;
///
/// let te = TestEnv::new();
/// let addr = te.env.register_contract(None, MyContract);
/// ```
pub struct TestEnv {
    pub env: Env,
}

impl TestEnv {
    /// Return a new [`TestEnv`] with mocked auths enabled and a realistic
    /// ledger timestamp so time-based contract logic does not special-case
    /// epoch zero.
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| {
            l.timestamp = DEFAULT_LEDGER_TIMESTAMP;
            l.sequence_number = DEFAULT_SEQUENCE;
        });
        TestEnv { env }
    }

    /// Advance the ledger timestamp by `seconds`.
    pub fn advance_time(&self, seconds: u64) {
        self.env.ledger().with_mut(|l| {
            l.timestamp = l.timestamp.saturating_add(seconds);
        });
    }

    /// Advance the ledger sequence number by `n`.
    pub fn advance_sequence(&self, n: u32) {
        self.env.ledger().with_mut(|l| {
            l.sequence_number = l.sequence_number.saturating_add(n);
        });
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_has_non_zero_timestamp() {
        let te = TestEnv::new();
        assert!(te.env.ledger().timestamp() > 0);
    }

    #[test]
    fn advance_time_increases_timestamp() {
        let te = TestEnv::new();
        let before = te.env.ledger().timestamp();
        te.advance_time(3600);
        assert_eq!(te.env.ledger().timestamp(), before + 3600);
    }

    #[test]
    fn advance_sequence_increases_sequence() {
        let te = TestEnv::new();
        let before = te.env.ledger().sequence();
        te.advance_sequence(10);
        assert_eq!(te.env.ledger().sequence(), before + 10);
    }
}
