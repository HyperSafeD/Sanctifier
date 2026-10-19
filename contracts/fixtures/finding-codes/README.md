# Finding Code Fixture Contracts

This directory contains fixture source files used as deterministic scan inputs for Sanctifier finding codes.

## Goals

- Keep one fixture per core `S***` finding code.
- Keep fixtures intentionally small and readable.
- Preserve stable scanner input text for contributor docs and manual verification.

## Fixture index

| Finding code | Fixture file                    |
| ------------ | ------------------------------- |
| `S001`       | `s001_authentication.rs`        |
| `S002`       | `s002_panic_handling.rs`        |
| `S003`       | `s003_arithmetic.rs`            |
| `S004`       | `s004_storage_limits.rs`        |
| `S005`       | `s005_storage_keys.rs`          |
| `S006`       | `s006_unsafe_patterns.rs`       |
| `S007`       | `s007_custom_rule.rs`           |
| `S008`       | `s008_events.rs`                |
| `S009`       | `s009_logic_result_handling.rs` |
| `S010`       | `s010_upgrade_admin.rs`         |
| `S011`       | `s011_formal_verification.rs`   |
| `S012`       | `s012_token_interface.rs`       |
| `S013`       | `s013_reentrancy.rs`            |
| `S014`       | `s014_admin_trust.rs`           |
| `S015`       | `s015_secrets.rs`               |
| `S016`       | `s016_truncation.rs`            |
| `S018`       | `s018_unsafe_prng.rs`           |
| `S019`       | `s019_unchecked_calls.rs`       |
| `S020`       | `s020_missing_events.rs`        |
| `S021`       | `s021_storage_misuse.rs`        |
| `S022`       | `s022_raw_invoke_contract.rs`   |
| `S025`       | `s025_missing_ttl_bump.rs`      |
| `S026`       | `s026_taint_propagation.rs`     |
| `S027`       | `s027_static_reentrancy.rs`     |
| `S030`       | `s030_require_auth_for_args.rs` |

## ZK fixture index

Z-rule fixtures follow the same `z0NN_description.rs` naming convention as the
`S***` fixtures above. Each fixture is annotated inline with which
function(s) trigger the rule (❌) and which demonstrate the clean, non-triggering
case (✅). Some fixtures hold both cases in one file; others (e.g. `Z001`) hold
only the triggering case today, with the clean counterpart tracked for a
future fixture.

| Finding code | Fixture file                                | Case                    |
| ------------ | -------------------------------------------- | ----------------------- |
| `Z001`       | `z001_missing_nullifier.rs`                  | Triggering only         |
| `Z002`       | `z002_insecure_randomness.rs`                | Triggering + clean      |
| `Z003`       | `z003_missing_public_input_binding.rs`        | Triggering + clean      |
| `Z004`       | `z004_unverified_trusted_setup.rs`            | Triggering + clean      |
| `Z005`       | `z005_missing_vk_integrity_check.rs`          | Triggering + clean      |
| `Z009`       | `z009_unbounded_verify_loop.rs`               | Triggering + clean      |
| `Z010`       | `z010_missing_vk_rotation_access_control.rs`  | Triggering + clean      |

The `Z003`–`Z005` fixtures are exercised directly by the snapshot suite
(`tooling/sanctifier-core/tests/sarif_snapshots.rs`), which asserts the exact set
of functions or constants each rule flags — so a fixture and its rule cannot drift
apart silently.

Rule definitions live under [`docs/rules/`](../../../docs/rules/); see
`Z001.md`–`Z014.md` for the full Z-rule catalog, and
[`docs/zk-roadmap.md`](../../../docs/zk-roadmap.md) for what ships in this wave.
This table is updated incrementally as each Z-rule fixture lands (see #1197–#1210,
#1217, #1218, #1222, #1223).

## Usage

From repository root:

```bash
sanctifier analyze contracts/fixtures/finding-codes --format json
```

These files are fixture sources, not deployable production contracts.
