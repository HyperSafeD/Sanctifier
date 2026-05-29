# Contract Versioning Conventions — Fixture Catalog

This document describes the versioning conventions used across `contracts/*` and
the named fixtures available in the versioning test suites.

---

## Convention

Every contract exposes:

| Symbol | Type | Description |
|---|---|---|
| `CONTRACT_VERSION` | `pub const u32` | Current schema version compiled into the binary |
| `get_version(env)` | `pub fn → u32` | Reads the stored version; returns `CONTRACT_VERSION` when the key is absent (pre-versioning deployments implied as v1) |
| `migrate(env, from)` _(AmmPool only)_ | `pub fn → bool` | Advances the stored version from `from` to `from + 1`; returns `false` when `from` doesn't match, preventing replays |

The version key is written to **instance storage** so it is co-located with the
rest of the contract's singleton state and shares its TTL.

### When to bump `CONTRACT_VERSION`

Bump the constant (and open a migration PR) when:
- A new `DataKey` variant is added with a mandatory default value.
- An existing storage value changes its serialized type.
- A new mandatory field is added to a stored struct.

Do **not** bump for additive-only changes where old readers can safely ignore
the new key.

---

## Fixture Reference

### AmmPool (`contracts/amm-pool/tests/versioning_tests.rs`)

| Fixture | Description | Version |
|---|---|---|
| `fixture_empty_pool` | Registered contract, no `add_liquidity` called, Version key absent | implicit 1 |
| `fixture_v1_funded_pool` | 4 000 × 9 000 initial deposit; Version key stamped `1` | 1 |
| `fixture_v2_pool` | `fixture_v1_funded_pool` after one successful `migrate(1)` | 2 |

### RuntimeGuardWrapper (`contracts/runtime-guard-wrapper/tests/versioning_tests.rs`)

| Fixture | Description | Version |
|---|---|---|
| `fixture_uninitialized` | Registered contract, `init` not yet called, Version key absent | implicit 1 |
| `fixture_initialized` | `init` called; Version key stamped `1` | 1 |
| `fixture_active` | `fixture_initialized` + 3 guarded calls (ping, echo, sum) | 1 |

---

## Test Coverage Matrix

| Test | AmmPool | RuntimeGuardWrapper |
|---|:---:|:---:|
| `CONTRACT_VERSION` constant equals 1 | ✓ | ✓ |
| Version key stamped on initialization | ✓ | ✓ |
| `get_version` matches constant | ✓ | ✓ |
| Version readable before any domain operations | ✓ | ✓ |
| Re-initialization is idempotent (version unchanged) | — | ✓ |
| Normal operations do not mutate version | ✓ | ✓ |
| `migrate` advances version by 1 | ✓ | — |
| `migrate` rejects wrong `from_version` | ✓ | — |
| `migrate` cannot be replayed after success | ✓ | — |
| Sequential migration chain v1→v2→v3→v4 | ✓ | — |
| Pre-versioning deployment fallback (no stored key) | ✓ | ✓ |

---

## Adding a New Version

1. Increment `CONTRACT_VERSION` in `src/lib.rs`.
2. If the contract has `migrate()`, ensure the new schema logic runs inside the
   version gate.
3. Add a `fixture_vN_pool` / `fixture_vN_wrapper` entry in the versioning test
   file mirroring the pattern above.
4. Add a row to the coverage matrix in this file.
5. Update `DOCUMENTATION_INDEX.md` if a new migration guide is added under
   `docs/`.

---

## Related Documentation

- [DOCUMENTATION_INDEX.md](../../../DOCUMENTATION_INDEX.md) — top-level index
- [contracts/README.md](../../README.md) — per-contract ownership map
- [docs/wasm-versioning-alignment.md](../../../docs/wasm-versioning-alignment.md) — WASM module versioning strategy
