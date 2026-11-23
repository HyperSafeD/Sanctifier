# M-of-N Multisig Wallet (`contracts/multisig`)

`contracts/multisig` (crate `multisig-wallet`) is a reference **M-of-N
multi-signature wallet** for Soroban. A fixed set of signers proposes,
approves and executes arbitrary cross-contract calls once enough of them have
approved. It also manages its own signer set and threshold through the same
approval process, and has a guardian-gated, timelocked **recovery** path for
the case where signers are lost.

Unlike [`allowance-race`](./allowance-race.md), this is a **reference design, not
a deliberately vulnerable fixture**. Its
counterpart is [`contracts/multisig-no-recovery`](../contracts/multisig-no-recovery/src/lib.rs),
which is the intentionally broken variant: no signer rotation, no recovery, and
approvals kept in `temporary` storage. Reading the two side by side shows what
the recovery path and persistent storage are for.

> **Status:** the contract carries its own "not audited, formal verification
> required" disclaimer (security level *Critical*; see
> [`contract-security-disclaimers.md`](./contract-security-disclaimers.md)).
> Treat it as a starting point, not production code.

## How a proposal moves

```
              anyone                 each signer (require_auth)            anyone
   propose(target, fn, args, salt) ─► approve(signer, hash) ×M ─► execute(target, fn, args, salt)
        │                                   │                          │
        ▼                                   ▼                          ▼
   hash = sha256(target‖fn‖args‖salt)   approval_count += 1       executed = true, THEN the call
   ProposalInfo { 0, false, false }     Approval(hash,signer)=true  → event "executed"
```

- A proposal is identified by the SHA-256 of its `target`, `function`, `args`
  and `salt` (XDR-encoded). Changing any of them gives a different proposal.
  The **salt** is what lets you propose the same call twice.
- `propose` is idempotent: proposing an existing hash returns it unchanged.
- `execute` re-derives the hash from its arguments, so what runs is exactly
  what was approved.

## The contract at a glance

| Function | Who can call | What it does |
|---|---|---|
| `init(signers, threshold)` | anyone, once | Stores the signer list and threshold. Rejects more than `MAX_SIGNERS` (20), a zero threshold, or a threshold above the signer count. |
| `propose(target, function, args, salt)` | **anyone** (no auth) | Creates the proposal, emits `proposed`, returns its hash. |
| `approve(signer, hash)` | a listed signer (`require_auth`) | Adds one approval. Rejects non-signers, unknown, executed or cancelled proposals, and a second approval from the same signer. |
| `execute(target, function, args, salt)` | **anyone** (no auth) | Runs the call once `approval_count >= threshold`. |
| `cancel(hash)`, `add_signer`, `remove_signer`, `set_threshold` | the wallet itself (`current_contract_address().require_auth()`) | Governance actions. In practice they run **through `execute`** (next section). |
| `set_recovery_guardian(guardian)` | the wallet itself | Sets the recovery guardian. |
| `initiate_recovery(guardian, new_signers, new_threshold)` | the guardian | Starts a recovery with a 7-day timelock. One pending recovery at a time. |
| `execute_recovery()` | **anyone**, after the timelock | Replaces the signers and threshold with the recovery request's values. |
| `cancel_recovery(caller)` | the guardian or the wallet itself | Drops a pending recovery. |
| `get_recovery_request()` | anyone (read-only) | The pending recovery, if any. |
| `get_security_disclaimer`, `validate_security_config` | anyone | Helpers from the shared `security-disclaimers` crate. |

### Governance goes through `execute`

When `execute` is called with `target == the wallet itself`, it does **not**
make a real cross-contract call. It routes natively to the internal helper for
`add_signer`, `remove_signer`, `set_threshold` or `cancel` (the source comment
says this avoids recursion and authorisation problems), and any other function
name panics with `InvalidArguments`. Any other `target` is invoked with
`env.invoke_contract`. The helpers enforce the invariants:

- adding an existing signer is a no-op, and the list never exceeds `MAX_SIGNERS`;
- `remove_signer` refuses to leave fewer signers than the threshold;
- `set_threshold` must be between 1 and the signer count.

### Recovery

The guardian does not hold funds or sign proposals. It exists for the
stuck-state case (lost keys, so the threshold can never be met again):

1. The guardian calls `initiate_recovery` with a new signer set and threshold.
   `unlock_at = now + 7 days`.
2. For seven days the request is visible through `get_recovery_request`.
3. After `unlock_at`, **anyone** can call `execute_recovery`, which overwrites
   the signers and threshold.
4. Until then the guardian (or the wallet itself) can `cancel_recovery`.

## Storage, events and errors

| Key | Storage | Value |
|---|---|---|
| `Signers`, `Threshold` | instance | The signer list and M |
| `RecoveryGuardian`, `RecoveryRequest` | instance | Guardian address; pending `RecoveryRequest { new_signers, new_threshold, unlock_at }` |
| `Proposal(hash)` | **persistent** | `ProposalInfo { approval_count, executed, cancelled }` |
| `Approval(hash, signer)` | **persistent** | `true` once that signer has approved |

Events: `proposed (hash) → (target, function)`, `approved (hash) → signer`,
`executed (hash) → target`, `cancelled (hash) → ()`. `init`, signer changes and
the whole recovery path emit **no** events.

Errors (`contracterror`, 15 codes): `NotInitialized` 1, `AlreadyInitialized` 2,
`InvalidThreshold` 3, `InsufficientSigners` 4, `Unauthorized` 5,
`ProposalNotFound` 6, `AlreadyApproved` 7, `ThresholdNotMet` 8,
`AlreadyExecuted` 9, `AlreadyCancelled` 10, `InvalidArguments` 11,
`TimelockActive` 12, `NoRecoveryGuardian` 13, `RecoveryAlreadyPending` 14,
`TooManySigners` 15. (`InsufficientSigners` is declared but never returned.)

## Security properties it demonstrates

1. **What runs is what was approved.** The proposal hash covers target,
   function, arguments and salt.
2. **No double approval.** `Approval(hash, signer)` is checked before counting.
3. **Execute-once, state before call.** `execute` sets `executed = true` and
   saves it *before* making the external call, so a re-entrant `execute` of the
   same proposal sees `AlreadyExecuted`.
4. **The signer set cannot be shrunk below the threshold** by `remove_signer`,
   and the threshold cannot be set above the signer count.
5. **Bounded work.** `MAX_SIGNERS = 20` bounds the `contains` scan in `approve`.
6. **A recovery path with a delay** (versus `multisig-no-recovery`), and
   **persistent** proposal storage so approvals do not expire mid-vote.

## Things to know when reading the code

These come from reading `lib.rs`; none of them is flagged as a bug by the
contract's own comments.

- **`propose`, `execute` and `execute_recovery` need no authentication.**
  This is deliberate in the multisig model (the security comes from the
  approvals), but it means anyone can create proposal entries, and anyone can
  trigger execution once the threshold is met.
- **`init` does not authenticate anyone,** so whoever calls it first chooses
  the signers. Initialise in the same transaction as deployment.
- **Approvals are a counter.** `approval_count` is incremented on approve and
  never recomputed, and removing a signer does not remove their
  `Approval(...)` entries or decrement any count. An approval from a signer who
  has since been removed therefore still counts toward the threshold of a
  still-pending proposal. Cancel and re-propose (with a new salt) after
  changing the signer set.
- **`init` does not reject duplicate signers.** A duplicate cannot approve
  twice, but it can make a threshold harder to reach than the signer count
  suggests.
- **The guardian is highly trusted.** After seven days an uncancelled recovery
  replaces *all* signers. Use a guardian you could not be coerced into
  misusing, and monitor `get_recovery_request`. There is no event to watch.
- **No TTL management.** The source never extends the TTL of its instance or
  persistent entries, so long-lived wallets need an external bump.
- **Recovery set-up is worth testing end to end.** `set_recovery_guardian`
  and the "wallet itself" branch of `cancel_recovery` require the wallet's own
  auth, while `execute`'s self-call routing only handles `add_signer`,
  `remove_signer`, `set_threshold` and `cancel`. As far as the source shows,
  `execute` cannot route a self-call to `set_recovery_guardian` or
  `cancel_recovery` (it would return `InvalidArguments`), and the existing
  tests all run under `env.mock_all_auths()`, which grants those self-auth
  checks automatically. **I have not run this against the real auth rules**, so
  treat it as a question to confirm with a test that does not mock auth.
- **Arithmetic is plain.** `info.approval_count += 1` and the recovery
  `timestamp() + 7 * 24 * 60 * 60` are unchecked additions. The crate sets
  `overflow-checks = true` for release builds, so an overflow would panic
  rather than wrap, and neither value gets near the limit in practice.

## Which Sanctifier rules look at it

The contract is listed in `contracts/README.md` as a *multi-party authorization
fixture*. These are the constructs in the source that Sanctifier's rules look
at, and what to expect when you read a report:

| Rule (catalog code) | What to look for in this contract |
|---|---|
| `auth_gap` (S001) | State-changing public functions with no `require_auth()` of their own: `init`, `propose`, `execute`, `execute_recovery`. The first three are discussed above. The analyzer does not know that approval counting is the real gate. |
| `panic_detection` (S002) | `env.panic_with_error(..)` throughout, plus `.unwrap()` on storage reads in `approve`, `execute` and the `internal_*` helpers. |
| arithmetic (S003) | The two unchecked additions above. |
| `unhandled_result` (S009) | Review any hit against the actual return type before acting on it. |
| `taint_propagation` (S026) | Caller-supplied `target` / `function` / `args` flow into `invoke_contract` inside `execute`; the approval count is the guard. |
| missing TTL bump (S025) | Persistent proposal/approval entries and instance data with no `extend_ttl`. |

> **I did not run Sanctifier for this page.** The table lists what the source
> contains, not a measured result. On another reference contract
> (`contracts/proxy`, see PR #1940) a real run showed some rules firing on sound
> code, for example `auth_gap` does not follow calls into helper functions, so
> expect to triage this report rather than take it at face value. Run it as
> below and compare.

## Reproduce a report

From the repository root:

```bash
# Build the CLI (Z3 is only needed for the S011 invariant check)
cargo build -p sanctifier-cli --no-default-features

# Human-readable report
./target/debug/sanctifier analyze contracts/multisig

# Findings per rule
./target/debug/sanctifier analyze contracts/multisig --format json \
  | jq -c '.findings | group_by(.rule) | map({rule: .[0].rule, count: length})[]'

# Compare with the intentionally vulnerable variant
./target/debug/sanctifier analyze contracts/multisig-no-recovery
```

Use `--format sarif` for an editor- or CI-friendly report. Build and test the
contract itself with `cargo build -p multisig-wallet` and
`cargo test -p multisig-wallet` (the tests live in `contracts/multisig/src/test.rs`).

## Using the pattern safely

1. Pick `M` and `N` so that losing `N - M + 1` keys is survivable, and set a
   guardian you trust *before* you need it.
2. Initialise atomically with deployment, and use distinct signer addresses.
3. After any signer change, re-propose pending actions rather than relying on
   approvals collected under the old signer set.
4. Watch `proposed` / `approved` / `executed` events for unexpected activity,
   and poll `get_recovery_request` for a pending recovery.
5. Bump the TTL of the wallet's storage as part of operations.

## Further reading

- [`contracts/multisig/src/lib.rs`](../contracts/multisig/src/lib.rs): the contract.
- [`contracts/multisig-no-recovery`](../contracts/multisig-no-recovery/src/lib.rs): the stuck-state variant.
- [`allowance-race.md`](./allowance-race.md): the vulnerable-fixture page this one is modeled on.
- [`getting-started.md`](./getting-started.md): installing and running Sanctifier.
