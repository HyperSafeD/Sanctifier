# Reentrancy Guard

`reentrancy-guard` is a small Soroban reference module that enforces a single
contract-local mutex using the type-safe `StorageKey::Guard` `#[contracttype]`
enum variant in instance storage.

## Invariant

- At most one re-entrant call is possible; once the guard is locked, every
  subsequent nested call reverts until the current execution exits.
- The mutex uses `StorageKey::Guard`, avoiding string/symbol key collisions with
  application-owned storage entries.
- The protection is contract-local only. It is not a cross-contract lock and
  does not synchronize state across different contract addresses.

## Fuzzing

This crate includes `cargo fuzz` harnesses under `fuzz/fuzz_targets` to exercise
unexpected sequences of states and transitions.

Install `cargo-fuzz` if it is not already available:

```bash
cargo install cargo-fuzz
```

Then from `contracts/reentrancy-guard`, run a target such as:

```bash
cargo fuzz run fuzz_guard_state_machine
cargo fuzz run fuzz_enter_exit_sequences -- -max_len=1024
cargo fuzz run fuzz_concurrent_access -- -max_len=256 -runs=10000000
```

These harnesses validate the core invariants of the guard by fuzzing arbitrary
status values and operation sequences, including edge cases around re-entrant
entry, exit, and repeated state transitions.

## Benchmarks

Run the benchmark with:

```bash
cargo bench -p reentrancy-guard --bench reentrancy_bench
```

The benchmark measures the per-invocation overhead of calling
`ReentrancyGuard::enter()` followed by `ReentrancyGuard::exit()` inside a
contract frame.

Sample local result:

```text
guard_enter_exit_per_invocation
                        time:   [18.235 us 18.889 us 19.661 us]
```

Treat the numbers as machine-specific. The command above should be used for the
current environment when comparing changes.
