//! Shared test helpers for Sanctifier example contracts.
//!
//! # Why this crate exists
//!
//! Every contract in `contracts/` needs the same Soroban test boilerplate:
//! a fresh `Env`, mocked authorizations, a token client for payments, and
//! assertions on ledger state.  Without a shared crate, Cargo compiles that
//! boilerplate separately for each test binary, which inflates CI compile
//! times roughly in proportion to the number of contracts in the workspace.
//!
//! Extracting the helpers here lets Cargo compile them once, cache the rlib,
//! and link it into every contract's test binary without recompilation.
//!
//! # Module layout
//!
//! | Module  | Contents |
//! |---------|----------|
//! | `env`   | [`TestEnv`] — pre-configured Soroban test environment |
//! | `token` | [`create_token`] — deploy and mint a native token in tests |
//! | `asserts` | Common assertion helpers |

pub mod asserts;
pub mod env;
pub mod token;
