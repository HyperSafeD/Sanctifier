# Quick Start: Runtime Guard Wrapper Deployment

Get up and running with automated Soroban runtime guard deployment in 10 minutes.

## 📋 Checklist

- [ ] Soroban CLI installed
- [ ] Secret key obtained
- [ ] GitHub secrets configured (optional, for CI/CD)
- [ ] Environment configured
- [ ] First deployment successful

## 🚀 5-Minute Setup

### Step 1: Get Your Secret Key (1 min)

Generate a Soroban keypair:

```bash
soroban keys generate --seed test-deployer --network testnet
```

Fund your account on testnet:

```bash
ACCOUNT=$(soroban keys show test-deployer)
curl "https://friendbot.stellar.org?addr=$ACCOUNT"
```

Get your secret key:

```bash
soroban keys show test-deployer --show-seed
# Output: SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### Step 2: Configure Environment (1 min)

```bash
cd /workspaces/Sanctifier

# Copy template
cp .env.example .env.local

# Edit with your secret key
nano .env.local

# Set this:
SOROBAN_SECRET_KEY=SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### Step 3: Test Dry Run (1 min)

```bash
# Source environment
source .env.local

# Run dry run (no actual deployment)
./scripts/deploy-soroban-testnet.sh --dry-run

# Expected output:
# ✓ Environment validated
# [DRY RUN] Contract would deploy: C...
```

### Step 4: Real Deployment (1 min)

```bash
# Deploy to testnet with validation
./scripts/deploy-soroban-testnet.sh --network testnet

# OR using CLI
sanctifier deploy contracts/runtime-guard-wrapper \
    --network testnet \
    --validate
```

### Step 5: Check Results (1 min)

```bash
# View deployment manifest
cat .deployment-manifest.json | jq '.'

# Get contract ID
CONTRACT_ID=$(jq -r '.deployments[0].contract_id' .deployment-manifest.json)

# Verify health
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network testnet \
    -- health_check
```

## 🔧 Common Commands

### Deployment

```bash
# Normal deployment
./scripts/deploy-soroban-testnet.sh --network testnet

# Dry run (no actual deployment)
./scripts/deploy-soroban-testnet.sh --dry-run

# Without continuous validation
./scripts/deploy-soroban-testnet.sh --no-continuous

# Custom validation interval (10 minutes)
./scripts/deploy-soroban-testnet.sh --interval 600

# Enable debug logging
./scripts/deploy-soroban-testnet.sh --debug

# CLI deployment
sanctifier deploy contracts/runtime-guard-wrapper \
    --network testnet \
    --validate
```

### Verification

```bash
# Check deployment status
cat .deployment-manifest.json | jq '.deployments[0]'

# Health check
soroban contract invoke \
    --id $CONTRACT_ID \
    --network testnet \
    -- health_check

# Get execution stats
soroban contract invoke \
    --id $CONTRACT_ID \
    --network testnet \
    -- get_stats

# View logs
tail -f .deployment.log
```

## 🔐 GitHub Actions (Optional)

### Add Secret

```bash
gh secret set SOROBAN_SECRET_KEY --body "YOUR_KEY_HERE"
```

### Trigger Deployment

1. Go to Actions tab
2. Select "Soroban Runtime Guard Deployment"
3. Click "Run workflow"
4. Select network (testnet)
5. Optional: Enable dry-run

### Or via CLI

```bash
gh workflow run soroban-deploy.yml \
    -f network=testnet \
    -f dry_run=false
```

## 📊 Outputs

### Deployment Manifest

File: `.deployment-manifest.json`

```json
{
  "deployments": [
    {
      "contract_id": "CXXXXX...",
      "name": "runtime-guard-wrapper",
      "status": "active",
      "deployed_at": "2026-02-25T12:34:56Z"
    }
  ]
}
```

### Deployment Log

File: `.deployment.log`

Contains detailed execution logs for debugging.

## 🆘 Troubleshooting

### "SOROBAN_SECRET_KEY not found"
```bash
# Set environment variable
export SOROBAN_SECRET_KEY="SBXXXXXXX..."
# Or source .env.local
source .env.local
```

### "soroban: command not found"
```bash
# Install Soroban CLI
cargo install --locked soroban-cli
```

### "WASM file not found"
```bash
# Build the contract
cargo build -p runtime-guard-wrapper \
    --release \
    --target wasm32-unknown-unknown
```

### "Contract deploy error"
```bash
# Check network
soroban network info --network testnet

# Check balance
soroban account balance --account $SOROBAN_ACCOUNT_ID --network testnet
```

## 📚 Next Steps

1. **Monitor Deployment**
   - Check logs: `tail -f .deployment.log`
   - Verify health: See [Verification](#verification) commands

2. **Continuous Validation**
   - Deployment runs validation automatically
   - Check stats every N seconds (configurable)
   - Results saved to manifest

3. **GitHub Actions**
   - Add SOROBAN_SECRET_KEY secret
   - Workflow triggers on push to main
   - Runs validation every 6 hours

4. **Production Setup**
   - Review [CI/CD Setup Guide](../docs/ci-cd-setup.md)
   - Configure branch protection
   - Set up production networks

## 📖 Documentation

### Quick References
- [CLI Deploy Command](../tooling/sanctifier-cli/README.md)
- [Bash Script Options](../scripts/README.md)

### In-Depth Guides
- [Full Deployment Guide](../docs/soroban-deployment.md)
- [CI/CD Setup](../docs/ci-cd-setup.md)
- [Runtime Guard Contract](../contracts/runtime-guard-wrapper/README.md)

### External Resources
- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Testnet Faucet](https://friendbot.stellar.org)

## 💡 Tips

✅ **Test First**
```bash
./scripts/deploy-soroban-testnet.sh --dry-run
```

✅ **Keep Track**
```bash
# Archive old manifests
mv .deployment-manifest.json "manifest-$(date +%s).json"
```

✅ **Monitor Regularly**
```bash
# Watch logs during deployment
watch -n 1 'tail -5 .deployment.log'
```

✅ **Rotate Credentials**
```bash
# Periodically update your secret key
gh secret set SOROBAN_SECRET_KEY --body "NEW_KEY_HERE"
```

## 🎯 What's Next?

After successfully deploying:

1. ✅ Review deployment manifest
2. ✅ Check health status
3. ✅ Monitor validation loop
4. ✅ Set up GitHub Actions (optional)
5. ✅ Configure branch protection (production)
6. ✅ Plan continuous validation schedule

---

**Last Updated:** February 25, 2026
**Time to Deploy:** ~5 minutes
**Difficulty:** ⭐ Easy
