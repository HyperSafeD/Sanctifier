# Deployment Configuration Reference

This document provides a complete reference for all configuration options available when deploying runtime guard wrapper contracts.

## Table of Contents

1. [Environment Variables](#environment-variables)
2. [Deployment Script Options](#deployment-script-options)
3. [CLI Tool Options](#cli-tool-options)
4. [GitHub Actions Configuration](#github-actions-configuration)
5. [Validation Configuration](#validation-configuration)

---

## Environment Variables

### Required Variables

#### SOROBAN_SECRET_KEY
The secret key for your Soroban account (wallet).

```bash
export SOROBAN_SECRET_KEY="SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
```

- **Format**: Base32-encoded secret key starting with `S`
- **Required**: Yes, for production deployments
- **Security**: Store in `.env.local` (not version controlled) or GitHub Secrets
- **How to get**: 
  ```bash
  soroban keys generate --seed <name> --network testnet
  soroban keys show <name> --reveal
  ```

### Optional Variables

#### SOROBAN_NETWORK
The blockchain network to use.

```bash
export SOROBAN_NETWORK=testnet
```

- **Values**: `testnet`, `futurenet`, `mainnet`
- **Default**: `testnet`
- **Recommended**: `testnet` for development

#### SOROBAN_RPC_URL
Custom RPC endpoint URL (advanced users).

```bash
export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
```

- **Default**: Auto-detected based on SOROBAN_NETWORK
- **Custom**: Use for private/custom RPC endpoints

#### SOROBAN_RPC_TIMEOUT
RPC request timeout in seconds.

```bash
export SOROBAN_RPC_TIMEOUT=30
```

- **Default**: `30`
- **Range**: `10-300`
- **Use**: Increase if experiencing timeout issues

#### DEBUG
Enable debug logging for troubleshooting.

```bash
export DEBUG=true
```

- **Values**: `true`, `false`
- **Default**: `false`
- **Output**: Verbose logging to console and logs

#### LOG_LEVEL
Logging level for deployment process.

```bash
export LOG_LEVEL=debug
```

- **Values**: `debug`, `info`, `warning`, `error`
- **Default**: `info`

---

## Deployment Script Options

### Script: `scripts/deploy-soroban-testnet.sh`

#### --network NETWORK
Specify target blockchain network.

```bash
bash scripts/deploy-soroban-testnet.sh --network testnet
```

- **Options**: `testnet`, `futurenet`, `mainnet`
- **Default**: `testnet`
- **Required**: No
- **Example**: Deployed to futurenet: `--network futurenet`

#### --no-validate
Skip validation after deployment.

```bash
bash scripts/deploy-soroban-testnet.sh --no-validate
```

- **Effect**: Disables post-deployment validation
- **Default**: Validation enabled
- **Use case**: Speed up deployment if you'll validate separately

#### --no-continuous
Disable continuous validation loop.

```bash
bash scripts/deploy-soroban-testnet.sh --no-continuous
```

- **Effect**: Exits after deployment, no ongoing validation
- **Default**: Continuous validation enabled
- **Use case**: CI/CD pipelines with separate validation jobs

#### --dry-run
Perform deployment without actually sending to blockchain.

```bash
bash scripts/deploy-soroban-testnet.sh --dry-run
```

- **Effect**: Builds and tests, but doesn't deploy
- **Default**: Disabled
- **Use case**: Test configuration before real deployment

#### --interval SECONDS
Set validation check interval.

```bash
bash scripts/deploy-soroban-testnet.sh --interval 600
```

- **Default**: `300` (5 minutes)
- **Range**: `60-3600`
- **Unit**: Seconds
- **Example**: Validate every 10 minutes: `--interval 600`

#### --debug
Enable debug logging.

```bash
bash scripts/deploy-soroban-testnet.sh --debug
```

- **Effect**: Enables verbose output
- **Default**: Disabled
- **Output**: To `$DEPLOYMENT_LOG`

#### --help
Display help information.

```bash
bash scripts/deploy-soroban-testnet.sh --help
```

### Script Environment Variables

These can be set to customize script behavior:

```bash
# Maximum deployment retry attempts
export MAX_RETRIES=5

# Validation interval (seconds)
export VALIDATION_INTERVAL=300

# Skip non-essential checks
export SKIP_CARGO_CHECK=false
```

---

## CLI Tool Options

### Command: `sanctifier deploy`

#### POSITIONAL ARGUMENTS

```bash
sanctifier deploy <CONTRACT_PATH>
```

- **Default**: `.` (current directory)
- **Required**: No
- **Example**: `./contracts/runtime-guard-wrapper`

#### --network NETWORK
Target blockchain network.

```bash
sanctifier deploy . --network testnet
```

- **Options**: `testnet`, `futurenet`, `mainnet`
- **Default**: `testnet`
- **Short flag**: `-n`

#### --secret-key KEY
Soroban secret key for deployment.

```bash
sanctifier deploy . --secret-key "$SOROBAN_SECRET_KEY"
```

- **Alternative**: Environment variable `SOROBAN_SECRET_KEY`
- **Short flag**: `-s`
- **Security**: Prefer environment variable

#### --account-id ID
Account ID for the deployer (optional).

```bash
sanctifier deploy . --account-id GXXXXX...
```

- **Format**: Stellar public key
- **Optional**: Usually auto-detected
- **Short flag**: `-a`

#### --validate
Enable validation after deployment.

```bash
sanctifier deploy . --validate
```

- **Effect**: Runs health check after deployment
- **Default**: Enabled
- **Short flag**: `-v`

#### --output-format FORMAT
Output format for results.

```bash
sanctifier deploy . --output-format json
```

- **Options**: `text`, `json`
- **Default**: `text`
- **Short flag**: `-o`
- **Use case**: `json` for parsing in scripts

### CLI Examples

```bash
# Basic deployment to testnet
sanctifier deploy ./contracts/runtime-guard-wrapper

# Deploy with validation and JSON output
sanctifier deploy ./contracts/runtime-guard-wrapper \
  --validate \
  --output-format json

# Deploy to mainnet (careful!)
sanctifier deploy ./contracts/runtime-guard-wrapper \
  --network mainnet \
  --secret-key "$MAINNET_KEY"
```

---

## GitHub Actions Configuration

### Workflow File: `.github/workflows/soroban-deploy.yml`

#### Trigger Configuration

```yaml
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
        description: "Target network"
      dry_run:
        default: false
        description: "Perform dry run"
```

#### Environment Variables

```yaml
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  DEBUG: false
  LOG_LEVEL: info
```

#### Job Configuration

Configure in workflow file under `jobs:` → `build-and-deploy:` → `steps:`

```yaml
- name: Deploy to Soroban testnet
  run: |
    bash scripts/deploy-soroban-testnet.sh \
      --network testnet \
      --interval 300 \
      --debug
  env:
    SOROBAN_SECRET_KEY: ${{ secrets.SOROBAN_SECRET_KEY }}
```

### Secrets Configuration

Add to **Settings** → **Secrets and variables** → **Actions**:

```
Name: SOROBAN_SECRET_KEY
Value: SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

### Scheduled Validation

The workflow runs continuous validation every 6 hours:

```yaml
schedule:
  - cron: "0 0,6,12,18 * * *"  # Every 6 hours
```

To change frequency, edit the cron expression:

- Every 2 hours: `"0 */2 * * *"`
- Every 12 hours: `"0 0,12 * * *"`
- Daily: `"0 0 * * *"`

See [cron syntax reference](https://crontab.guru/) for more patterns.

---

## Validation Configuration

### Script: `scripts/validate-runtime-guards.sh`

#### --contract-id ID
Contract ID to validate (required).

```bash
bash scripts/validate-runtime-guards.sh \
  --contract-id C1234567890123456789012345678901234567890123456789012345
```

- **Format**: Contract address starting with `C`
- **Required**: Yes
- **Short flag**: None

#### --network NETWORK
Target network for validation.

```bash
bash scripts/validate-runtime-guards.sh \
  --contract-id C... \
  --network testnet
```

- **Options**: `testnet`, `futurenet`, `mainnet`
- **Default**: `testnet`
- **Short flag**: None

### Validation Test Configuration

Edit test parameters in script:

```bash
# In validate-runtime-guards.sh

# Performance threshold (milliseconds)
PERF_THRESHOLD=30000

# Minimum pass rate to succeed
MIN_PASS_RATE=80

# Concurrent operation count
CONCURRENT_OPS=3
```

### Test Suite

The validation script runs these tests (by default, all enabled):

1. **Health Check** - Verify contract accessibility
2. **Statistics** - Retrieve and validate stats
3. **Execution Monitoring** - Test contract invocations
4. **Event Emission** - Verify event logging
5. **Storage Access** - Check storage accessibility
6. **Performance** - Measure execution time
7. **Error Handling** - Test error conditions
8. **Concurrent Ops** - Test concurrent calls

---

## Configuration Examples

### Development Deployment

```bash
# .env.local
SOROBAN_SECRET_KEY=SBXXXXXXX...
SOROBAN_NETWORK=testnet
DEBUG=true
LOG_LEVEL=debug

# Deploy with all features
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --interval 60 \
  --debug
```

### CI/CD Deployment

```yaml
# .github/workflows/soroban-deploy.yml
- name: Deploy
  run: |
    bash scripts/deploy-soroban-testnet.sh \
      --network testnet \
      --no-continuous
```

### Quick Deployment (No Validation)

```bash
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --no-validate \
  --no-continuous
```

### Production-like (with Extensive Validation)

```bash
bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --interval 300 \
  --validate \
  --debug
```

---

## Configuration Precedence

Settings are applied in this order (later overrides earlier):

1. **Script defaults**
2. **Environment variables**
3. **CLI/script arguments**
4. **GitHub Actions secrets/inputs**

---

## Validation Results

Results are saved to:

- **Manifest**: `.deployment-manifest.json`
- **Logs**: `.deployment.log`
- **Validation**: `.validation-results.json`

Example results structure:

```json
{
  "contract_id": "C1234567...",
  "network": "testnet",
  "timestamp": "2024-02-25T10:30:00Z",
  "test_results": {
    "total_tests": 8,
    "passed": 7,
    "failed": 1,
    "pass_rate": 87
  },
  "status": "PASS"
}
```

---

## Troubleshooting Configuration

Enable full debugging:

```bash
export DEBUG=true
export LOG_LEVEL=debug

bash scripts/deploy-soroban-testnet.sh \
  --network testnet \
  --dry-run \
  --debug
```

Check output files:

```bash
# Deployment log
tail -200 .deployment.log

# Manifest
jq . .deployment-manifest.json

# Validation results
jq . .validation-results.json
```

---

For more information, see:
- [Soroban Deployment Guide](./soroban-deployment.md)
- [Environment Setup Guide](./.env.example)
- [Sanctifier Documentation](./README.md)
