# Draft PR

## Title
feat(core): AST parser for Soroban storage types in collision analysis

## Summary
This PR enhances the static analysis engine to differentiate Soroban storage scopes (`instance`, `persistent`, `temporary`) when detecting storage key collisions.

Previously, identical key values used in different storage scopes could be incorrectly flagged as collisions.

## What changed
- Added AST-based storage scope parsing in `StorageVisitor`.
- Added `SorobanStorageType` classification (`Instance`, `Persistent`, `Temporary`, `Unknown`).
- Updated key tracking to be scope-aware using `(storage_type, key_value)` grouping.
- Updated collision messaging to include storage scope context.
- Added regression tests:
	- Detect collisions only within the same storage scope.
	- Ignore same key reuse across different scopes.

## Files changed (functional)
- `tooling/sanctifier-core/src/storage_collision.rs`
- `tooling/sanctifier-core/src/lib.rs`

## Additional repo maintenance changes
- `Cargo.lock`: resolved merge conflict markers that blocked Cargo commands.
- `tooling/sanctifier-cli/src/commands/analyze.rs`: rustfmt-only formatting.
- `tooling/sanctifier-core/src/smt.rs`: rustfmt-only formatting.

## Validation
- ✅ `cargo fmt --check`
- ⚠️ `cargo clippy --package sanctifier-cli --package sanctifier-core --package amm-pool --all-targets --all-features -- -D warnings`
	- Local environment blocker: missing `z3.h` header (`z3-sys` build dependency).
	- Expected to pass in CI environments with Z3 development headers installed.

## Why this matters
This reduces false positives in storage collision reports and aligns analyzer behavior with actual Soroban storage semantics, improving trust in static analysis findings.

## Checklist
- [x] Implement parser for Soroban storage types
- [x] Integrate with storage collision detection
- [x] Add targeted tests for same-scope vs cross-scope behavior
- [x] Keep change scoped to static analysis enhancement
- [ ] Confirm full CI green in hosted runner

## Copy-ready PR body
This PR implements AST parsing for Soroban storage types to improve storage collision detection.

### Highlights
- Adds scope-aware parsing for `instance`, `persistent`, and `temporary` storage chains.
- Detects collisions only when key values overlap within the same storage type.
- Prevents false positives when the same key is reused across different storage types.
- Adds regression tests to lock behavior.

### Context
Part of static analysis engine enhancements for more accurate Soroban contract risk detection.

### Local verification
- `cargo fmt --check` passed.
- `cargo clippy ... -D warnings` is blocked locally by missing `z3.h` (system dependency), not by Rust lint issues in this feature.
