# Soroban Runtime Guard Deployment Guide

## Overview

This guide covers the complete process for automating deployment of runtime guard wrapper contracts to the Soroban testnet for continuous validation.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Setup](#environment-setup)
3. [Local Deployment](#local-deployment)
4. [CI/CD Setup](#cicd-setup)
5. [Continuous Validation](#continuous-validation)
6. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Tools

- **Rust**: 1.70+ with WebAssembly target
- **Soroban CLI**: Latest version
- **Cargo**: Rust package manager
- **curl**: For API interactions
- **jq**: For JSON parsing
- **git**: Version control

### Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Add WebAssembly target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli

# Verify installations
rustup --version
cargo --version
soroban --version
```

### Account Setup

1. **Generate a test account:**
   ```bash
   soroban keys generate --seed test-deployer --network testnet
   ```

2. **Fund the account on testnet:**
   ```bash
   ACCOUNT=$(soroban keys show test-deployer)
   curl "https://friendbot.stellar.org?addr=$ACCOUNT"
   ```

3. **Verify balance:**
   ```bash
   soroban account balance --account test-deployer --network testnet
   ```

---

## Environment Setup

### 1. Local Development Environment

Create a `.env.local` file in the project root:

```bash
cp .env.example .env.local
```

Edit `.env.local` with your credentials:

```bash
# Your Soroban secret key
export SOROBAN_SECRET_KEY="SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# Network configuration
export SOROBAN_NETWORK=testnet
export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org

# Deployment settings
export DEPLOYMENT_NETWORK=testnet
export VALIDATION_INTERVAL=300
export MAX_RETRIES=3
```

Load the environment:

```bash
source .env.local
```

### 2. GitHub Actions Secrets

1. Go to your repository: **Settings** → **Secrets and variables** → **Actions**

2. Create a new secret:
   - **Name**: `SOROBAN_SECRET_KEY`
   - **Value**: Your actual secret key (starts with `S`)

Using GitHub CLI:

```bash
# Get your secret key
SECRET_KEY=$(soroban keys show test-deployer --reveal)

# Add to GitHub
gh secret set SOROBAN_SECRET_KEY --body "$SECRET_KEY"
```

---

## Local Deployment

### Build Runtime Guard Wrapper

```bash
# Navigate to project root
cd /workspaces/Sanctifier

# Build the wrapper contract
cargo build -p runtime-guard-wrapper --release --target wasm32-unknown-unknown

# Verify WASM artifact
ls -lah target/wasm32-unknown-unknown/release/runtime_guard_wrapper.wasm
```

### Manual Deployment

Using the CLI tool:

```bash
# Deploy with validation
sanctifier deploy ./contracts/runtime-guard-wrapper \
  --network testnet \
  --secret-key "$SOROBAN_SECRET_KEY" \
  --validate

# Deploy without validation
sanctifier deploy ./contracts/runtime-guard-wrapper \
  --network testnet \
  --secret-key "$SOROBAN_SECRET_KEY"
```

### Using Deployment Script

```bash
# Standard deployment
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --interval 300

# Dry run (no actual deployment)
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --dry-run

# Without continuous validation
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --no-continuous

# With debug logging
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --debug
```

### Monitor Deployment

Check the deployment logs:

```bash
# View deployment manifest
cat .deployment-manifest.json | jq .

# View deployment log
tail -f .deployment.log
```

---

## CI/CD Setup

### GitHub Actions Workflow

The workflow file is located at: `.github/workflows/soroban-deploy.yml`

#### Triggers

The workflow runs on:

1. **Push to main branch** (if contract files changed)
2. **Schedule** (every 6 hours for continuous validation)
3. **Manual trigger** (via `workflow_dispatch`)

#### Configuration

Edit the workflow file to customize:

```yaml
# In .github/workflows/soroban-deploy.yml

on:
  push:
    branches: ["main"]
    paths:
      - "contracts/runtime-guard-wrapper/**"
  schedule:
    - cron: "0 */6 * * *"  # Every 6 hours
  workflow_dispatch:
    inputs:
      network:
        default: "testnet"
      dry_run:
        default: false
```

#### Workflow Stages

1. **Build & Test**
   - Checkout code
   - Install Rust toolchain
   - Build runtime guard wrapper
   - Verify WASM artifact

2. **Deploy**
   - Run deployment script
   - Collect deployment manifest
   - Upload artifacts

3. **Continuous Validation**
   - Download deployment manifest
   - Run validation tests
   - Generate validation report

4. **Notification**
   - Create deployment status check
   - Post summary to README

### View Workflow Runs

```bash
# List recent workflow runs
gh run list --workflow soroban-deploy.yml

# View latest run details
gh run view --workflow soroban-deploy.yml

# View logs for a specific run
gh run view <RUN_ID> --log
```

---

## Continuous Validation

### Validation Tests

The [`validate-runtime-guards.sh`](../scripts/validate-runtime-guards.sh) script runs:

1. **Health Check** - Verifies contract operations
2. **Statistics** - Retrieves contract stats
3. **Execution Monitoring** - Tests guarded execution
4. **Event Emission** - Validates event logs
5. **Storage Accessibility** - Checks storage access
6. **Performance Baseline** - Measures execution time
7. **Error Handling** - Tests error conditions
8. **Concurrent Operations** - Tests concurrent calls

### Manual Validation

```bash
# Run validation suite for a deployed contract
bash scripts/validate-runtime-guards.sh \
  --contract-id C1234567890123456789012345678901234567890123456789012345 \
  --network testnet

# View validation results
cat .validation-results.json | jq .
```

### Continuous Validation Loop

The deployment script includes a built-in validation loop:

```bash
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --interval 300 \
  --no-validate false
```

This will:
- Deploy contracts
- Run validation tests every 300 seconds
- Continue running indefinitely
- Log all results

### Monitor Validation

```bash
# Follow validation log in real-time
tail -f .deployment.log

# Check deployment manifest for status
jq '.deployments[] | {name, status, last_validated}' .deployment-manifest.json
```

---

## Troubleshooting

### Build Issues

**Error: "target wasm32-unknown-unknown not found"**

```bash
# Install the WebAssembly target
rustup target add wasm32-unknown-unknown
```

**Error: "soroban command not found"**

```bash
# Install Soroban CLI
cargo install --locked soroban-cli

# Verify installation
soroban --version
```

### Deployment Issues

**Error: "SOROBAN_SECRET_KEY not found"**

```bash
# Check environment variable
echo $SOROBAN_SECRET_KEY

# Set it if missing
export SOROBAN_SECRET_KEY="SBXXXXXXXX..."
```

**Error: "Insufficient balance"**

```bash
# Fund your account on testnet
ACCOUNT=$(soroban keys show test-deployer)
curl "https://friendbot.stellar.org?addr=$ACCOUNT"

# Check balance
soroban account balance --account test-deployer --network testnet
```

**Error: "RPC connection failed"**

```bash
# Check network is accessible
soroban network list
soroban network info --network testnet

# Test RPC endpoint
curl -s https://soroban-testnet.stellar.org/health
```

### Validation Issues

**Error: "Contract health check failed"**

```bash
# Check contract is deployed
soroban contract read \
  --id C1234567... \
  --network testnet

# Invoke health check directly
soroban contract invoke \
  --id C1234567... \
  --network testnet \
  -- health_check
```

### GitHub Actions Issues

**Workflow not triggering on push**

- Check branch protection rules
- Verify secrets are set correctly
- Check workflow file syntax with: `gh workflow list`

**"Error: Secret not found"**

```bash
# Verify secret exists
gh secret list

# Update secret if needed
gh secret set SOROBAN_SECRET_KEY --body "new-value"
```

---

## Best Practices

### Security

1. ✅ Never commit `.env.local` to version control
2. ✅ Use GitHub Secrets for CI/CD credentials
3. ✅ Rotate keys regularly
4. ✅ Use separate accounts for testnet/mainnet
5. ✅ Review deployment logs regularly
6. ✅ Validate deployments before promoting to mainnet

### Deployment

1. ✅ Always do a dry run first: `--dry-run`
2. ✅ Test on testnet before mainnet
3. ✅ Enable continuous validation
4. ✅ Monitor deployment logs
5. ✅ Keep deployment manifests for audit trail
6. ✅ Document all deployments

### Validation

1. ✅ Run full validation suite after deployment
2. ✅ Enable continuous validation loop
3. ✅ Set appropriate validation intervals
4. ✅ Monitor validation reports
5. ✅ Alert on failed validations
6. ✅ Review performance metrics

---

## Next Steps

1. **Set up local environment** (follow Environment Setup)
2. **Test local deployment** with `--dry-run`
3. **Configure GitHub Secrets**
4. **Push changes to trigger workflow**
5. **Monitor deployment** in Actions tab
6. **Validate deployed contracts**
7. **Review logs and manifests**
8. **Setup alerts** for failed deployments

## Resources

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar CLI Reference](https://github.com/stellar/stellar-cli)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Sanctifier Documentation](./README.md)

---

For questions or issues, refer to the [Troubleshooting](#troubleshooting) section or check the [Sanctifier Issues](https://github.com/HyperSafeD/Sanctifier/issues).
