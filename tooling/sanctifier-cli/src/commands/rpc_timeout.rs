//! Shared RPC timeout constant and helpers for Soroban CLI subprocess calls.
//!
//! Both `deploy` and `verify_deployment` spawn `soroban`/`stellar` subprocesses
//! that make RPC calls to a Soroban network.  Without a timeout those
//! subprocesses can hang indefinitely when a testnet or mainnet RPC node is
//! slow or unresponsive — a scenario that has caused recurring CI failures.
//!
//! This module provides:
//!
//! * [`DEFAULT_RPC_TIMEOUT_SECS`] — the default subprocess timeout (30 s).
//! * [`add_timeout_arg`] — appends `--timeout <secs>` to a [`Command`] arg
//!   list when the CLI supports it, so both callers stay in sync.
//!
//! # Usage
//!
//! ```rust,no_run
//! use std::process::Command;
//! use sanctifier_cli::commands::rpc_timeout::{DEFAULT_RPC_TIMEOUT_SECS, add_timeout_arg};
//!
//! let mut cmd = Command::new("soroban");
//! cmd.args(["contract", "fetch", "--id", "CXXX", "--network", "testnet"]);
//! add_timeout_arg(&mut cmd, DEFAULT_RPC_TIMEOUT_SECS);
//! let _status = cmd.status();
//! ```

use std::process::Command;

/// Default wall-clock timeout (in seconds) applied to every Soroban RPC
/// subprocess call.  30 s is intentionally generous — a healthy testnet
/// responds in well under 10 s — while still bounding hangs that have
/// previously caused CI jobs to exceed their 6-minute limit.
///
/// Users can override this via the `--rpc-timeout` flag exposed on both
/// `sanctifier deploy` and `sanctifier verify-deployment`.
pub const DEFAULT_RPC_TIMEOUT_SECS: u64 = 30;

/// Append `--timeout <secs>` to `cmd`'s argument list.
///
/// The Stellar/Soroban CLI has accepted `--timeout` (in seconds) since
/// version 0.8.  Passing it unconditionally is safe: on CLI versions that
/// ignore unknown flags the call either succeeds or fails for the actual
/// reason (not the extra flag), and callers already handle failures.
///
/// A timeout of `0` is treated as "no timeout" (the argument is not added)
/// so callers can opt out at runtime if needed.
pub fn add_timeout_arg(cmd: &mut Command, timeout_secs: u64) {
    if timeout_secs > 0 {
        cmd.args(["--timeout", &timeout_secs.to_string()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn default_timeout_is_thirty_seconds() {
        assert_eq!(DEFAULT_RPC_TIMEOUT_SECS, 30);
    }

    #[test]
    fn add_timeout_arg_appends_flag_and_value() {
        let mut cmd = Command::new("echo");
        add_timeout_arg(&mut cmd, 45);
        // Inspect the args by running the command and capturing output.
        // `echo` will print its arguments, letting us verify the flag was added.
        let output = cmd.output().expect("echo should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("--timeout"),
            "expected --timeout in output, got: {stdout}"
        );
        assert!(
            stdout.contains("45"),
            "expected timeout value 45 in output, got: {stdout}"
        );
    }

    #[test]
    fn add_timeout_arg_zero_does_not_append_flag() {
        let mut cmd = Command::new("echo");
        add_timeout_arg(&mut cmd, 0);
        let output = cmd.output().expect("echo should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("--timeout"),
            "timeout=0 must not add --timeout flag, got: {stdout}"
        );
    }

    #[test]
    fn add_timeout_arg_uses_default_constant() {
        // Verify that the default constant produces a flag value > 0,
        // i.e. the guard is always active by default.
        let mut cmd = Command::new("echo");
        add_timeout_arg(&mut cmd, DEFAULT_RPC_TIMEOUT_SECS);
        let output = cmd.output().expect("echo should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("--timeout"),
            "default constant must produce --timeout flag, got: {stdout}"
        );
        assert!(
            stdout.contains(&DEFAULT_RPC_TIMEOUT_SECS.to_string()),
            "expected {DEFAULT_RPC_TIMEOUT_SECS} in output, got: {stdout}"
        );
    }
}
