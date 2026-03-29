.PHONY: build test lint fmt audit release clean docs

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

## Auto-format all workspace source files.
fmt:
	cargo fmt --all

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
