# Contract ABI / Interface Reference

This document is the canonical reference for the public ABI of every contract
in `contracts/*`.  It is kept in sync with the source by the
`make contract-docs-check` CI step.

The machine-readable version lives at
[`docs/generated/contract-interfaces.json`](generated/contract-interfaces.json)
(schema version 1).

---

## Regenerating docs

```bash
# Generate rustdoc HTML + refresh the JSON summary
make contract-docs

# Verify the committed JSON matches the current source (CI mode)
make contract-docs-check
```

Rustdoc output lands in `target/doc/`.  Open
`target/doc/<crate_name>/index.html` in a browser to browse the full API.

---

## amm-pool

Constant-product AMM (x·y = k) for two tokens.

| Function | Signature | Description |
|---|---|---|
| `add_liquidity` | `(token_a, token_b, amount_a, amount_b, min_lp) -> u128` | Deposit token pair, receive LP tokens |
| `remove_liquidity` | `(lp_amount, min_a, min_b) -> (u128, u128)` | Burn LP tokens, receive proportional reserves |
| `swap` | `(token_in, amount_in, min_out) -> u128` | Swap one token for the other |
| `get_price` | `(token_in, token_out) -> u128` | Spot price scaled by 1 000 000 |

Error codes: `ZeroAmount(1)` `IdenticalTokens(2)` `InvalidPair(3)`
`PoolNotInitialized(4)` `InsufficientLiquidity(5)` `SlippageExceeded(6)`
`MintBelowMinimum(7)` `LockedLiquidity(8)` `ArithmeticOverflow(9)`

ABI stability tests: `contracts/amm-pool/tests/integration_tests.rs`
(`abi_*` test functions).

---

## governance-contract

Token-weighted on-chain governor with configurable quorum, threshold, and
timelock integration.

| Function | Description |
|---|---|
| `init` | One-time initialisation |
| `propose` | Submit a new proposal, returns proposal id |
| `cast_vote` | Vote for (1) / against (0) / abstain (2) |
| `queue` | Queue a succeeded proposal in the timelock |
| `execute` | Execute a queued proposal |
| `state` | Query the current `ProposalState` |

Error codes: `NotInitialized(1)` `AlreadyInitialized(2)` `Unauthorized(3)`
`ProposalNotFound(4)` `InvalidState(5)` `InvalidVote(6)` `AlreadyVoted(7)`
`QuorumNotMet(8)` `ProposalThresholdNotMet(9)`

---

## multisig-wallet

M-of-N multisignature wallet.

| Function | Description |
|---|---|
| `init` | Initialise with signers list and threshold |
| `propose` | Create a proposal, returns its hash |
| `approve` | Approve a proposal (signer auth required) |
| `execute` | Execute once threshold is met |
| `cancel` | Cancel a pending proposal (contract auth) |
| `add_signer` | Add a signer (contract auth) |
| `remove_signer` | Remove a signer (contract auth) |
| `set_threshold` | Update approval threshold (contract auth) |

Error codes: `NotInitialized(1)` `AlreadyInitialized(2)` `InvalidThreshold(3)`
`InsufficientSigners(4)` `Unauthorized(5)` `ProposalNotFound(6)`
`AlreadyApproved(7)` `ThresholdNotMet(8)` `AlreadyExecuted(9)`
`AlreadyCancelled(10)` `InvalidArguments(11)`

---

## timelock

Role-based timelock controller.

| Function | Description |
|---|---|
| `init` | One-time initialisation |
| `get_min_delay` | Query minimum delay in seconds |
| `is_proposer` / `is_executor` / `is_canceller` | Role queries |
| `set_proposer` / `set_executor` / `set_canceller` | Grant/revoke roles (admin) |
| `update_delay` | Change minimum delay (admin) |
| `schedule` | Schedule a call with a delay |
| `execute` | Execute a ready scheduled call |
| `cancel` | Cancel a pending scheduled call |

Error codes: `AlreadyInitialized(1)` `NotInitialized(2)` `Unauthorized(3)`
`InsufficientDelay(4)` `ProposalNotFound(5)` `ProposalNotReady(6)`
`InvalidDelay(8)`

---

## reentrancy-guard

Formally-verified single-contract reentrancy mutex.

| Function | Description |
|---|---|
| `enter` | Acquire the lock (errors if already locked) |
| `exit` | Release the lock |
| `new` | Construct a guard instance |

No contract error enum — reentrancy is signalled via `panic`.

---

## runtime-guard-wrapper

Wraps another contract with pre/post execution guards and metrics.

| Function | Description |
|---|---|
| `init` | Initialise with wrapped contract address |
| `execute_guarded` | Call a function on the wrapped contract with guards |
| `get_stats` | Return execution statistics |
| `get_wrapped_contract` | Return the wrapped contract address |
| `health_check` | Run a health check on the wrapped contract |

---

## my-contract (SEP-41 Token)

Reference SEP-41 fungible token implementation.

| Function | Description |
|---|---|
| `initialize` | One-time setup (admin, decimals, name, symbol) |
| `mint` | Mint tokens to an address (admin only) |
| `burn` | Burn tokens from caller's balance |
| `burn_from` | Burn tokens using an allowance |
| `transfer` | Transfer tokens to another address |
| `transfer_from` | Transfer using an allowance |
| `approve` | Set an allowance with expiry ledger |
| `allowance` | Query current allowance |
| `balance` | Query token balance |
| `decimals` / `name` / `symbol` | Metadata queries |

Error codes: `AlreadyInitialized(1)` `NotInitialized(2)`
`InsufficientBalance(3)` `InsufficientAllowance(4)` `AllowanceExpired(5)`
`Overflow(6)` `NegativeAmount(7)`

---

## Output format stability

The JSON schema is versioned at `schema_version: "1"`.  Any breaking change
to the schema (field removal, type change) requires:

1. Bumping `schema_version`.
2. A migration note in `CHANGELOG.md`.
3. Updating this document.

Adding new fields is non-breaking and does not require a version bump.
