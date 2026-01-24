# Contracts

This directory contains the Soroban smart contracts for the project.

## Structure
- `vulnerable-contract/`: A reference implementation demonstrating common security pitfalls Sanctifier can detect.
- `guarded-contract/`: A secure implementation using Sanctifier's runtime guards.

## Development

```bash
cd vulnerable-contract
cargo test
```

## Security
Run the Sanctifier analysis tool on these contracts:
```bash
sanctifier analyze .
```
