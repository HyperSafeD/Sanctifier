.PHONY: build test lint fmt audit release clean docs contract-docs contract-docs-check

## Build all workspace crates (debug).
build:
	cargo build --workspace

## Run all workspace tests.
test:
	cargo test --workspace

## Check formatting and run Clippy with -D warnings.
lint:
	cargo fmt --all --check
	cargo clippy --workspace -- -D warnings
	npm install && npm run format:db:check && npm run lint:db && npm run lint:release-artifacts

## Auto-format all workspace source files.
fmt:
	cargo fmt --all
	npm install && npm run format:db

## Run cargo-audit and cargo-deny supply-chain checks.
audit:
	cargo audit
	cargo deny check

## Build all workspace crates in release mode.
release:
	cargo build --workspace --release

## Generate and open rustdoc for all workspace crates (no deps).
docs:
	cargo doc --workspace --no-deps --open

## Remove all build artefacts.
clean:
	cargo clean

## Generate ABI / interface docs for all contracts (rustdoc + JSON summary).
contract-docs:
	bash scripts/gen-contract-docs.sh

## Verify contract-interfaces.json is up-to-date (used in CI).
contract-docs-check:
	bash scripts/gen-contract-docs.sh --check
