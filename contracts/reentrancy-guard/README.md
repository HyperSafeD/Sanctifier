# Reentrancy Guard

`reentrancy-guard` is a small Soroban reference module that enforces a single
contract-local mutex using the instance-storage key `RE_GRD`.

## Invariant

- At most one re-entrant call is possible; once the guard is locked, every
  subsequent nested call reverts until the current execution exits.
- The mutex is stored under the short symbol `RE_GRD`.
- The protection is contract-local only. It is not a cross-contract lock and
  does not synchronize state across different contract addresses.

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
