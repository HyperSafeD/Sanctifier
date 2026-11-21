# UUPS Upgrade Proxy (`contracts/proxy`)

`contracts/proxy` (crate `uups-proxy`) is a reference implementation of an
**admin-controlled, upgradeable Soroban contract** in the style of UUPS: the
upgrade logic lives *inside* the contract that is being upgraded, and only the
designated admin can trigger it.

Unlike [`allowance-race`](./allowance-race.md), this contract is **not a
deliberately vulnerable fixture**. It is a template for how an upgrade path
*should* look, so reading it alongside Sanctifier's output is mostly about
telling real findings from the ones the analyzer raises on a sound design.

> **Status:** the contract carries its own "not audited" disclaimer (see
> [`contract-security-disclaimers.md`](./contract-security-disclaimers.md)).
> Treat it as a starting point, not as production code.

## What the pattern is

On EVM, a UUPS proxy `delegatecall`s into a separate implementation contract.
**Soroban has no `delegatecall`.** Instead, a contract can replace its *own*
code:

```rust
env.deployer().update_current_contract_wasm(new_wasm);   // new_wasm: BytesN<32>
```

So there is a single contract. "Upgrading" means pointing it at a different,
already-uploaded WASM, identified by its hash. Storage stays where it is, and
the source comment says the new code replaces the old "atomically at the end of the transaction". The `Design`
diagram in the source's module comment uses the EVM proxy/implementation
vocabulary; read it as an analogy, not as two deployed contracts.

```
admin ──upgrade(new_wasm_hash)──► UupsProxy ──update_current_contract_wasm──► same contract ID,
                                   │                                          new code (end of tx)
                                   ├─ bumps VERSION
                                   ├─ records IMPLHASH
                                   └─ emits "upgraded"
```

## The contract at a glance

| Function | Auth | What it does |
|---|---|---|
| `initialize(admin, impl_hash)` | **none** (see [Known limitations](#known-limitations)) | One-time setup: stores `admin`, sets `VERSION = 1`, records `impl_hash`, bumps TTL, emits `init`. Panics with `already initialised` on a second call. |
| `upgrade(new_wasm)` | admin | Swaps the contract's WASM, then `VERSION += 1` (`checked_add`), records `new_wasm` as `IMPLHASH`, bumps TTL, emits `upgraded`. |
| `transfer_admin(new_admin)` | admin | Step 1 of a two-step hand-over: stores `new_admin` as pending, emits `adm_nom`. Calling it again replaces the nomination. |
| `accept_admin()` | the **pending** admin | Step 2: pending admin signs, becomes `ADMIN`, the pending entry is removed, emits `adm_xfer`. Panics if nothing is pending. |
| `admin()`, `pending_admin()`, `version()`, `impl_hash()` | none (read-only) | Views. `admin()` / `impl_hash()` panic if uninitialised; `version()` returns `0`. |
| `get_security_disclaimer(category)`, `validate_security_config(has_admin, has_upgrade)` | none | Helpers from the shared [`security-disclaimers`](./contract-security-disclaimers.md) crate. Without the `security-disclaimers` feature they return a fixed string / `has_admin`. |

**Storage** (all instance storage; the lifecycle functions above extend its TTL
to about a month whenever it falls under about a week, at 5 s per ledger):

| Constant | Symbol | Type | Meaning |
|---|---|---|---|
| `ADMIN` | `ADMIN` | `Address` | Current administrator |
| `PEND_ADMIN` | `PENDADMIN` | `Address` | Nominated, not yet accepted |
| `VERSION` | `VERSION` | `u32` | Upgrade counter, starts at 1 |
| `IMPL_HASH` | `IMPLHASH` | `BytesN<32>` | Hash passed to the latest `initialize`/`upgrade` |
| `INITIALISED` | `INIT` | `bool` | Re-initialisation guard |

**Events:** `init (admin, 1)`, `upgraded (version, wasm_hash)`,
`adm_nom (new_admin)`, `adm_xfer (new_admin)`. Watching `upgraded` is how
off-chain monitors notice a code swap.

## Security properties it demonstrates

1. **Only the admin can upgrade or hand over control.** `upgrade` and
   `transfer_admin` both go through the private `require_admin` helper, which
   loads `ADMIN` and calls `admin.require_auth()`.
2. **Upgrades are observable.** Every successful upgrade emits `upgraded` and
   updates `VERSION` and `IMPLHASH`, so an unexpected change is visible
   on-chain.
3. **Admin transfer cannot strand the contract.** The nominee has to sign
   `accept_admin` themselves. Nominating an address that cannot sign simply
   leaves `ADMIN` unchanged.
4. **No silent re-initialisation.** `initialize` is guarded by the `INIT` flag,
   so a deployed contract's admin cannot be reset by calling it again.
5. **The version counter cannot wrap.** `upgrade` increments it with
   `checked_add(..).expect("version overflow")`, and a Kani harness
   (`verify_version_increases`, compiled only under `cfg(kani)`) states the
   monotonicity property for `before < u32::MAX`.

## How Sanctifier sees it

Running the CLI on `contracts/proxy` reports **20 findings across five
rules**. None of them is a deliberately planted bug, so the useful skill here
is triage. The table separates what each finding actually means.

| Rule (JSON `rule`, catalog code) | Count | Where | Assessment |
|---|---|---|---|
| `auth_gap` (S001) | 3 | `initialize`, `upgrade`, `transfer_admin` | **`initialize`: real.** It writes `ADMIN` without any `require_auth()`. **`upgrade` / `transfer_admin`: reported, but guarded.** The check is in `require_admin`, a helper the rule does not follow. |
| `taint_propagation` (S026) | 4 | `initialize` (`admin`, `impl_hash`), `upgrade` (`new_wasm`), `transfer_admin` (`new_admin`) | **`initialize`: real**, for the same reason as above. **`upgrade` / `transfer_admin`: same helper limitation.** |
| `panic_detection` (S002) | 7 | `initialize`, `upgrade`, `accept_admin`, `admin`, `impl_hash`, `require_admin`, `verify_version_increases` | **Intentional.** These are `panic!` / `expect` on "already initialised", "not initialised", "no pending admin" and "version overflow". The last one is the `unwrap` inside the `cfg(kani)` proof harness, which the analyzer reads as ordinary source. |
| `unhandled_result` (S009) | 5 | `get_security_disclaimer` (×2), `validate_security_config`, `upgrade`, `transfer_admin` | **Not real.** The flagged calls return `String`, `bool` and `()` (`Self::require_admin`), never a `Result`. |
| `unused_variable` | 1 | the `after` binding in the Kani harness | **Harness only.** Not contract code. |

Two further points matter when you read a report:

- The upgrade-pattern analysis (**S010**, `UPGRADE_RISK`) runs in the library
  and in the `sanctifier-wasm` analyzer, but in the run above the CLI's `analyze` output has no
  upgrade-pattern section. Run
  directly against this source it reports `initialize` as an initialisation
  function **with** a re-init guard (medium), plus a high `Governance` finding
  for `upgrade` and `transfer_admin` saying they modify state without
  `require_auth`. That is the same helper limitation as above.
- No event, storage-collision, ledger-size or TTL findings are reported,
  which matches the code: it emits events on every state change, uses five
  distinct short symbols, and extends TTL in the lifecycle functions.

The numbers above come from `sanctifier-cli` 0.1.0 built with
`--no-default-features` (that skips Z3, which only the S011 invariant check
needs). They will change as rules change, so re-run the commands below rather than
relying on this table.

## Known limitations

These are properties of the template itself, independent of the analyzer:

- **`initialize` does not authenticate `admin`.** Whoever calls it first
  chooses the admin. Initialise in the same transaction as deployment, or add
  `admin.require_auth();` as the first statement. Doing so removes the three
  `initialize` findings (20 to 17 in a local copy), leaving only the
  helper-indirection ones.
- **No timelock and no WASM allow-list.** `upgrade` accepts any hash the
  admin signs for, and `IMPLHASH` just records it. Use a multi-sig admin and see
  [`contracts/timelock`](../contracts/timelock) if you need a delay.
- **`ProxyError` is defined but never returned.** Failures are string panics, so
  the `Downgrade` and `Unauthorized` variants are documentation only. See
  [`error-codes.md`](./error-codes.md) for how Sanctifier's own codes are
  structured.
- **The module comment mentions a `UupsImpl` mixin.** No such type exists in the
  crate today. Only `UupsProxy` is defined.
- **No unit tests.** The crate has no `#[test]` functions, so
  `cargo test -p uups-proxy` (the command in `contracts/MIGRATION_NOTES.md`) has nothing
  to exercise. The only proof harness is the trivial Kani property above (see
  [`kani-integration.md`](./kani-integration.md)).

## Reproduce the findings

From the repository root:

```bash
# Build the CLI (no Z3 needed for these rules)
cargo build -p sanctifier-cli --no-default-features

# Human-readable report
./target/debug/sanctifier analyze contracts/proxy

# Count findings per rule
./target/debug/sanctifier analyze contracts/proxy --format json \
  | jq -c '.findings | group_by(.rule) | map({rule: .[0].rule, count: length})[]'
# {"rule":"auth_gap","count":3}
# {"rule":"panic_detection","count":7}
# {"rule":"taint_propagation","count":4}
# {"rule":"unhandled_result","count":5}
# {"rule":"unused_variable","count":1}

# Just the auth-gap locations
./target/debug/sanctifier analyze contracts/proxy --format json \
  | jq -r '.findings[] | select(.rule=="auth_gap") | .location'
# initialize:154
# upgrade:175
# transfer_admin:196
```

Use `--format sarif` for an editor- or CI-friendly report. To watch the real
finding go away, copy the crate, add `admin.require_auth();` at the top of
`initialize`, and run `analyze` on the copy.

Build the contract itself with `cargo build -p uups-proxy`.

## Using the pattern safely

1. Make the admin a multi-sig (see [`contracts/multisig`](../contracts/multisig))
   and put a delay in front of `upgrade`.
2. Authenticate `admin` inside `initialize`, or initialise atomically with
   deployment.
3. Monitor the `upgraded` and `adm_xfer` events and compare the new `IMPLHASH`
   with the hash in your release manifest. The
   [rollback procedure](../ROLLBACK_PROCEDURE.md) uses exactly this check.
4. Keep storage layouts backward compatible across versions. `VERSION` tells
   you which layout a stored value belongs to; it does not migrate anything for you.

## Further reading

- [`contracts/proxy/src/lib.rs`](../contracts/proxy/src/lib.rs): the contract.
- [`allowance-race.md`](./allowance-race.md): the vulnerable-fixture counterpart to this page.
- [`getting-started.md`](./getting-started.md): installing and running Sanctifier.
