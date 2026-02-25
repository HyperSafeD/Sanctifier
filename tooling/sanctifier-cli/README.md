# Sanctifier CLI - Deploy Command

Use the `deploy` command to build a Soroban contract and deploy it to a Soroban network (testnet/mainnet/local).

Examples

- Deploy using environment variable `SOROBAN_SECRET_KEY`:

```bash
export SOROBAN_SECRET_KEY="S...your secret..."
cargo build -p sanctifier-cli --release
./target/release/sanctifier deploy --path contracts/vulnerable-contract --network testnet
```

- Deploy by passing secret on the command line:

```bash
./target/release/sanctifier deploy --path contracts/vulnerable-contract --network testnet --secret S...
```

Notes

- The command uses the `soroban` CLI under the hood. Install it with:

```bash
cargo install --locked soroban-cli
```

- The tool will build the contract for the `wasm32-unknown-unknown` target. Ensure the target is installed:

```bash
rustup target add wasm32-unknown-unknown
```

If you want, I can add this README content to the top-level project `README.md` as well.
