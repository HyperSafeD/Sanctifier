//! Assertion helpers for contract test suites.

/// Assert that `result` is `Ok`, returning the inner value.
///
/// Panics with a descriptive message on `Err`.
#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => panic!("expected Ok, got Err: {:?}", e),
        }
    };
    ($result:expr, $msg:literal) => {
        match $result {
            Ok(v) => v,
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    };
}

/// Assert that `result` is `Err`.
///
/// Returns the error value for further inspection.
#[macro_export]
macro_rules! assert_err {
    ($result:expr) => {
        match $result {
            Err(e) => e,
            Ok(v) => panic!("expected Err, got Ok: {:?}", v),
        }
    };
    ($result:expr, $msg:literal) => {
        match $result {
            Err(e) => e,
            Ok(v) => panic!("{}: got Ok({:?})", $msg, v),
        }
    };
}

/// Check that two i128 amounts differ by at most `tolerance` units.
///
/// Useful for fee-adjusted amount comparisons.
pub fn assert_within(actual: i128, expected: i128, tolerance: i128) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "amount mismatch: expected {expected} ± {tolerance}, got {actual} (diff {diff})"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_within_passes_at_exact_value() {
        assert_within(100, 100, 0);
    }

    #[test]
    fn assert_within_passes_within_tolerance() {
        assert_within(102, 100, 5);
    }

    #[test]
    #[should_panic(expected = "amount mismatch")]
    fn assert_within_fails_outside_tolerance() {
        assert_within(110, 100, 5);
    }
}
