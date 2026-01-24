# Getting Started with Sanctifier

## Installation

```bash
cargo install --path sanctifier-cli
```

## Running Your First Scan

Navigate to your Soroban contract directory:

```bash
cd my-soroban-project
sanctifier analyze
```

## Interpreting Results

Sanctifier checks for:
1. **Auth Gaps**: Missing `require_auth` in state-modifying functions.
2. **Storage Collisions**: Identical keys resolving to the same storage slot.
3. **Gas Usage**: Estimates resource consumption.
