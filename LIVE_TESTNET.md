# Live Testnet Deployment

> Sanctifier's example contracts are deployed and operational on **Stellar Soroban Testnet**.
> Anyone can verify the deployment, query state, and inspect on-chain audit events.

📹 **New to Sanctifier?** Follow the [Live-Testnet Video Walkthrough Script](docs/VIDEO_WALKTHROUGH_SCRIPT.md) — a timestamped storyboard covering health checks, stats, live audit events, and on-chain verification (~8 min).

---

## Deployed Contract Addresses

| Contract | Address | Explorer | Status |
|---|---|---|---|
| **Runtime Guard Wrapper** | `CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB` | [view ↗](https://stellar.expert/explorer/testnet/contract/CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB) | ✅ Initialized · Recording calls |
| **Vulnerable Contract** (demo) | `CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB` | [view ↗](https://stellar.expert/explorer/testnet/contract/CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB) | ✅ Deployed |
| **Reentrancy Guard** | `CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY` | [view ↗](https://stellar.expert/explorer/testnet/contract/CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY) | ✅ Deployed |

**Deployer wallet:** `GC7ZDZPZS3NUDCCM6JRLF5DSGARKE5JH5DXDXCUUAJU2RL2C2UJKGUUW`
([view on Stellar Expert ↗](https://stellar.expert/explorer/testnet/account/GC7ZDZPZS3NUDCCM6JRLF5DSGARKE5JH5DXDXCUUAJU2RL2C2UJKGUUW))

**Network:** Soroban Testnet · `Test SDF Network ; September 2015`
**RPC:** `https://soroban-testnet.stellar.org`

---

## Verify the Deployment

### Health check the Runtime Guard Wrapper

```bash
stellar contract invoke \
  --id CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB \
  --source <your-key> \
  --network testnet \
  -- health_check
# expected: true
```

### Read aggregate statistics

```bash
stellar contract invoke \
  --id CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB \
  --source <your-key> \
  --network testnet \
  -- get_stats
# returns: [total_calls, successful_calls, failed_calls]
```

### Inspect a recorded call

```bash
stellar contract invoke \
  --id CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB \
  --source <your-key> \
  --network testnet \
  -- get_call --call_id 1
# returns the CallRecord stored at id=1
```

### View deployment transaction

The contract was deployed in transaction
[`765308c7…dd37eb7`](https://stellar.expert/explorer/testnet/tx/765308c7169ccee2150ab3a24f9a5caaef43d98cf309c966845f0b7b2dd37eb7)
and initialized in transaction
[`b2c9b1a9…637fec`](https://stellar.expert/explorer/testnet/tx/b2c9b1a981059e2a12dfe5da4580f3ff70240ea89b2fc1442712ca5701637fec).

---

## Re-deploy From Source

Anyone can build and re-deploy the same contracts from `main`:

```bash
# 1. Build all three contracts to wasm32v1-none
stellar contract build --package runtime-guard-wrapper
stellar contract build --package vulnerable-contract
stellar contract build --package reentrancy-guard

# 2. Deploy each (replace <your-key> with a funded testnet key)
stellar contract deploy \
  --wasm target/wasm32v1-none/release/runtime_guard_wrapper.wasm \
  --source <your-key> --network testnet

stellar contract deploy \
  --wasm target/wasm32v1-none/release/vulnerable_contract.wasm \
  --source <your-key> --network testnet

# 3. Initialize the wrapper
stellar contract invoke \
  --id <wrapper-id> --source <your-key> --network testnet \
  -- init --admin <your-addr> --wrapped_contract <vuln-addr>
```

Or use the automation script:

```bash
./scripts/deploy-soroban-testnet.sh --network testnet --validate
```

---

## What Reviewers Should See

- **Live contracts** that respond to `health_check`, `get_stats`, and `get_call` from any caller.
- **On-chain audit events** emitted by `record_call` — Stellar Expert shows them under the contract's Events tab.
- **A working deployer wallet** funded via Friendbot with sufficient testnet XLM for additional interaction.
- **Reproducible build** — `cargo test --workspace` and `stellar contract build` succeed locally and in CI.

---

## Known Limitations of This Deployment

- The wrapper is initialized with the deployer wallet as both `admin` and `wrapped_contract` for demonstration purposes. In production deployments, `wrapped_contract` should be the address of an actual target protocol.
- Storage TTL on testnet is shorter than mainnet. After ~30 days, archived state may need to be restored before reads succeed.
- These addresses are intended for review and demonstration. Production deployments will be announced via the LIVE_MAINNET.md document after security audit.

## Telegram Event Watcher

If you want Telegram alerts for testnet runtime-guard events, use the scaffold in [`integrations/telegram/README.md`](integrations/telegram/README.md). It supports severity and contract-address filtering and can read from a local JSON event cache.
