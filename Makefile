.PHONY: build test lint fmt audit release clean docs docs-publish contract-docs contract-docs-check dev-setup

## Build all workspace crates (debug).
build:
	cargo build --workspace

## Set up development environment with all prerequisites.
dev-setup:
	@echo "Setting up Sanctifier development environment..."
	@echo "Installing Rust toolchain..."
	rustup update stable
	rustup default stable
	rustup target add wasm32-unknown-unknown
	@echo "Installing wasm-pack..."
	cargo install wasm-pack
	@echo "Installing soroban-cli..."
	cargo install soroban-cli
	@echo "Installing Node.js dependencies..."
	npm install
	@echo "Development environment setup complete!"
	@echo "Next steps:"
	@echo "  1. Install Z3: see docs/frontend-setup.md for platform-specific instructions"
	@echo "  2. Run 'make build' to build the project"
	@echo "  3. Run 'make test' to run tests"

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

## Build docs for GitHub Pages deployment (no open).
docs-publish:
	cargo doc --workspace --no-deps --lib

## Remove all build artefacts.
clean:
	cargo clean

## Generate ABI / interface docs for all contracts (rustdoc + JSON summary).
contract-docs:
	bash scripts/gen-contract-docs.sh

## Verify contract-interfaces.json is up-to-date (used in CI).
contract-docs-check:
	bash scripts/gen-contract-docs.sh --check
