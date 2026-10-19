# Mainnet Migration Guide

> **Target audience:** Existing testnet users of Sanctifier CLI/dashboard who are ready to deploy and analyse contracts on Stellar Soroban Mainnet.

---

## Overview

Moving from testnet to mainnet involves several important changes to your workflow. This guide covers the configuration, security, and economic differences you need to be aware of.

---

## 1. Prerequisites

Before migrating, ensure you have:

- [ ] A funded mainnet Stellar account with at least enough XLM for contract deployment fees
- [ ] Sanctifier CLI v0.3.0 or later (run `sanctifier --version`)
- [ ] The network passphrase for mainnet: `Public Global Stellar Network ; September 2015`

---

## 2. Configuration Changes

### 2.1 Network Selection

**Testnet (old):**
```bash
sanctifier deploy --network testnet
```

**Mainnet (new):**
```bash
sanctifier deploy --network mainnet
```

### 2.2 Environment Variables

Update your `.env.local` or CI secrets:

| Variable | Testnet Value | Mainnet Value |
|---|---|---|
| `SOROBAN_NETWORK` | `testnet` | `mainnet` |
| `SOROBAN_RPC_URL` | `https://soroban-testnet.stellar.org` | `https://soroban.stellar.org` |
| `SOROBAN_SECRET_KEY` | Testnet dev key | Mainnet production key |
| `NETWORK_PASSPHRASE` | `Test SDF Network ; September 2015` | `Public Global Stellar Network ; September 2015` |

### 2.3 Deployment Scripts

Update your deploy scripts to reference mainnet:
```bash
# Before (testnet)
./scripts/deploy-soroban-testnet.sh --network testnet

# After (mainnet)
./scripts/deploy-soroban-testnet.sh --network mainnet
```

---

## 3. New Safety Guards

### 3.1 Network-Passphrase Guard (`--network-passphrase`)

A network-passphrase mismatch now prevents accidental cross-network deploys. Sanctifier verifies that the passphrase in your configuration matches the target network before executing any transaction.

- If the passphrase does not match, the CLI exits with an error explaining the mismatch.
- This prevents deploying contract code to mainnet when your configuration targets testnet (and vice versa).

**Usage:**
```bash
sanctifier deploy --network mainnet --network-passphrase "Public Global Stellar Network ; September 2015"
```

### 3.2 Confirm-Mainnet Flag (`--confirm-mainnet`)

When targeting mainnet, you **must** pass the `--confirm-mainnet` flag as an explicit acknowledgement:

```bash
sanctifier deploy --network mainnet --confirm-mainnet
```

This flag:
- Is required for any write operation on mainnet (deploy, invoke state-mutating calls).
- Acts as a human-in-the-loop check against accidental mainnet commands.
- Does not apply to read-only operations (health checks, stats queries).

### 3.3 Why These Guards Exist

Without these guards, a single mistyped `--network mainnet` in a CI script or terminal could:
- Deploy unaudited contracts to mainnet.
- Accidentally use testnet keys on mainnet.
- Incur real XLM fees for unintended operations.

---

## 4. Fee and Cost Implications

### 4.1 Transaction Fees

| Operation | Testnet | Mainnet |
|---|---|---|
| Contract deploy | Free | ~1–10 XLM (varies with WASM size) |
| Contract invoke (read) | Free | ~0.001–0.01 XLM |
| Contract invoke (write) | Free | ~0.001–0.05 XLM |
| Storage access | Free | ~0.0001 XLM per entry |

### 4.2 Ledger Entry Rental

Mainnet uses a rent-based storage model. Each ledger entry requires:
- **Initial rent payment** at deploy time (paid upfront for ~1 year by default).
- **Ongoing rent** — if the entry is not extended (TTL bump), it may be archived.

**Recommendation:** Set storage TTL to at least 1 year for production contracts. Use Sanctifier's TTL analysis rules to detect entries with short TTL.

### 4.3 Budgeting

Estimate your monthly mainnet costs:
1. Count average daily transaction volume.
2. Multiply by per-transaction fee (estimate 0.01 XLM as a conservative average).
3. Add initial deploy costs (~5–10 XLM per contract).
4. Add rent reserve (~2–5 XLM per active storage entry).

---

## 5. Key Management

### 5.1 Separate Keys per Network

- **Never reuse testnet secret keys on mainnet.**
- Generate a dedicated mainnet key pair using Stellar Laboratory or `stellar keys generate`.
- Store mainnet keys in a secure vault (e.g., GitHub Secrets, 1Password, AWS Secrets Manager).

### 5.2 CI/CD Recommendations

```yaml
# .github/workflows/deploy-mainnet.yml
jobs:
  deploy:
    steps:
      - run: sanctifier deploy \
          --network mainnet \
          --confirm-mainnet \
          --network-passphrase "Public Global Stellar Network ; September 2015"
        env:
          SOROBAN_SECRET_KEY: ${{ secrets.MAINNET_SECRET_KEY }}
```

---

## 6. Breaking Changes from Testnet

| Aspect | Testnet Behaviour | Mainnet Behaviour |
|---|---|---|
| Passphrase validation | Warns on mismatch | **Blocks** on mismatch |
| `--confirm-mainnet` | Not required | **Required** for writes |
| Fee estimation | Returns 0 | Returns real fee |
| TTL defaults | 30-day default | Must set explicitly |
| Wasm size limits | ~256 KB | ~128 KB (stricter) |

---

## 7. Verification Checklist

Before you consider the migration complete:

- [ ] Mainnet account funded with sufficient XLM
- [ ] `.env.local` or CI secrets updated to mainnet values
- [ ] Can run `sanctifier deploy --dry-run --network mainnet` without errors
- [ ] `--confirm-mainnet` flag added to all deploy scripts
- [ ] Network passphrase matches mainnet configuration
- [ ] Testnet and mainnet keys are different
- [ ] Fee budget calculated and funded
- [ ] Storage TTL configured for production retention

---

## 8. Rollback

If you encounter issues on mainnet:

1. Stop all mainnet deploy scripts immediately.
2. Revert environment variables to testnet values.
3. Run `sanctifier doctor` to verify testnet connectivity.
4. Open an issue at https://github.com/HyperSafeD/Sanctifier/issues.

---

**Last Updated:** July 2026  
**See also:** [GETTING_STARTED.md](GETTING_STARTED.md) · [LIVE_TESTNET.md](LIVE_TESTNET.md)
