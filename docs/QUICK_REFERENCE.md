# Deployment Quick Reference

## One-Liner Commands

### Build the Runtime Guard Wrapper
```bash
cargo build -p runtime-guard-wrapper --release --target wasm32-unknown-unknown
```

### Deploy to Testnet (Local)
```bash
bash scripts/deploy-soroban-testnet.sh --network testnet
```

### Deploy with CLI Tool
```bash
sanctifier deploy ./contracts/runtime-guard-wrapper --network testnet --validate
```

### Validate a Deployed Contract
```bash
bash scripts/validate-runtime-guards.sh --contract-id C1234567... --network testnet
```

### Dry Run (No Real Deployment)
```bash
bash scripts/deploy-soroban-testnet.sh --network testnet --dry-run
```

### View Deployment Status
```bash
cat .deployment-manifest.json | jq '.deployments[] | {name, contract_id, status}'
```

---

## Environment Setup

```bash
# Copy example
cp .env.example .env.local

# Edit with your secret key (obtain via: soroban keys generate)
export SOROBAN_SECRET_KEY="SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# Load environment
source .env.local
```

---

## GitHub Actions Setup

```bash
# Add secret to GitHub
gh secret set SOROBAN_SECRET_KEY --body "your-key-here"

# View workflow runs
gh run list --workflow soroban-deploy.yml

# View logs
gh run view <RUN_ID> --log
```

---

## Common Workflows

### Development Workflow
```bash
# 1. Make contract changes
# 2. Build locally
cargo build -p runtime-guard-wrapper --release --target wasm32-unknown-unknown

# 3. Deploy to testnet
bash scripts/deploy-soroban-testnet.sh --network testnet --debug

# 4. Validate
CONTRACT_ID=$(cat .deployment-manifest.json | jq -r '.deployments[0].contract_id')
bash scripts/validate-runtime-guards.sh --contract-id $CONTRACT_ID --network testnet

# 5. Review results
tail .deployment.log
cat .validation-results.json | jq .
```

### Testing Before Push
```bash
# Dry run to verify everything works
bash scripts/deploy-soroban-testnet.sh --network testnet --dry-run --debug

# If dry run succeeds, push to main to trigger CI/CD
git add -A
git commit -m "Deploy: schedule deployment"
git push origin main

# Watch CI/CD
gh run list --workflow soroban-deploy.yml --limit 1
```

### Monitoring Continuous Deployment
```bash
# Enable continuous validation
bash scripts/deploy-soroban-testnet.sh --network testnet --interval 300

# In another terminal, watch logs
watch -n 5 'tail -10 .deployment.log'

# Check validation results every hour
watch -n 3600 'cat .validation-results.json | jq .'
```

---

## Troubleshooting Commands

### Verify Setup
```bash
# Check tools
soroban --version
cargo --version
rustup target list | grep wasm32

# Check environment
echo $SOROBAN_SECRET_KEY | head -c 2
soroban network info --network testnet

# Check account
soroban account balance --account $(soroban keys show test-deployer) --network testnet
```

### If Build Fails
```bash
# Install WebAssembly target
rustup target add wasm32-unknown-unknown

# Clean and rebuild
cargo clean -p runtime-guard-wrapper
cargo build -p runtime-guard-wrapper --release --target wasm32-unknown-unknown
```

### If Deployment Times Out
```bash
# Check RPC endpoint
curl -s https://soroban-testnet.stellar.org/health

# Increase timeout in your environment
export SOROBAN_RPC_TIMEOUT=60

# Retry with more verbose output
bash scripts/deploy-soroban-testnet.sh --network testnet --debug
```

### If Validation Fails
```bash
# Check contract is accessible
soroban contract read --id C1234567... --network testnet

# Test health check manually
soroban contract invoke --id C1234567... --network testnet -- health_check

# Run validation with debug
bash scripts/validate-runtime-guards.sh --contract-id C1234567... --network testnet
```

---

## File Locations

| File | Purpose |
|------|---------|
| `.env.local` | Local environment variables (NOT version controlled) |
| `.env.example` | Template for environment setup |
| `.deployment-manifest.json` | Record of all deployments |
| `.deployment.log` | Deployment script logs |
| `.validation-results.json` | Test results from validation |
| `.validation.log` | Validation script logs |

---

## Environment Variables Cheat Sheet

```bash
# Required
SOROBAN_SECRET_KEY=SBXXXXXXX...        # Your secret key

# Optional
SOROBAN_NETWORK=testnet               # Network (default: testnet)
DEBUG=true                             # Enable debug logging
LOG_LEVEL=debug                        # Logging level
VALIDATION_INTERVAL=300                # Validation check interval (seconds)
MAX_RETRIES=3                          # Deployment retry attempts
```

---

## Script Options Cheat Sheet

### `deploy-soroban-testnet.sh`
| Option | Effect |
|--------|--------|
| `--network testnet` | Deploy to testnet |
| `--dry-run` | Test without deploying |
| `--no-validate` | Skip validation |
| `--no-continuous` | Exit after deploy (no loop) |
| `--interval 300` | Validate every 300 seconds |
| `--debug` | Verbose logging |

### `validate-runtime-guards.sh`
| Option | Effect |
|--------|--------|
| `--contract-id <ID>` | Contract to validate |
| `--network testnet` | Target network |

### `sanctifier deploy`
| Option | Effect |
|--------|--------|
| `--network testnet` | Deploy to testnet |
| `--secret-key KEY` | Use specified key |
| `--validate` | Run validation after deploy |
| `--output-format json` | JSON output |

---

## Common Issues & Fixes

| Issue | Fix |
|-------|-----|
| `Command not found: soroban` | `cargo install --locked soroban-cli` |
| `wasm32-unknown-unknown not found` | `rustup target add wasm32-unknown-unknown` |
| `SOROBAN_SECRET_KEY not found` | `export SOROBAN_SECRET_KEY="your-key"` |
| `Insufficient balance` | `curl https://friendbot.stellar.org?addr=YOUR_ADDR` |
| `RPC connection failed` | Check network: `soroban network info --network testnet` |
| `Deployment timeout` | Increase timeout: `export SOROBAN_RPC_TIMEOUT=60` |

---

## Full Documentation

For comprehensive documentation, see:
- [Soroban Deployment Guide](soroban-deployment.md)
- [Deployment Configuration Reference](deployment-config.md)
- [Implementation Summary](DEPLOYMENT_IMPLEMENTATION.md)

---

## Support

For issues or questions:
1. Check the troubleshooting section above
2. Review deployment logs: `tail .deployment.log`
3. Check GitHub Actions: `gh run view <RUN_ID> --log`
4. See full documentation in `docs/` directory
