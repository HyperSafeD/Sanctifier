# Soroban Runtime Guard Deployment Automation

This directory contains automation tooling for deploying runtime guard wrapper contracts to Soroban testnet with continuous validation.

## 📋 Overview

Sanctifier now provides automated deployment of runtime guard wrapper contracts that:
- **Wrap target contracts** with security guards
- **Validate state invariants** before and after execution
- **Collect execution metrics** for auditing
- **Monitor continuously** with health checks
- **Deploy via CLI or GitHub Actions** for CI/CD integration

## 🚀 Quick Start

### Local Deployment

```bash
# 1. Configure environment
cp .env.example .env.local
# Edit .env.local with your SOROBAN_SECRET_KEY

# 2. Source environment
source .env.local

# 3. Deploy with continuous validation
./scripts/deploy-soroban-testnet.sh --network testnet

# OR use the CLI
sanctifier deploy contracts/runtime-guard-wrapper \
    --network testnet \
    --validate
```

### GitHub Actions Deployment

1. **Add GitHub Secret:**
   ```bash
   gh secret set SOROBAN_SECRET_KEY --body "SBXXXXXXX..."
   ```

2. **Push to main or manually trigger:**
   - Actions tab > Soroban Runtime Guard Deployment > Run workflow

## 📁 Project Structure

```
Sanctifier/
├── contracts/runtime-guard-wrapper/     # Runtime guard wrapper contract
│   ├── Cargo.toml                        # Contract manifest
│   ├── src/lib.rs                        # Guard implementation
│   └── tests/integration_tests.rs        # Test harness
├── tooling/sanctifier-cli/
│   └── src/commands/deploy.rs            # Deploy command
├── scripts/
│   └── deploy-soroban-testnet.sh         # Deployment automation script
├── .github/workflows/
│   └── soroban-deploy.yml                # CI/CD workflow
├── docs/
│   └── soroban-deployment.md             # Full documentation
└── .env.example                          # Environment template
```

## 🔧 Components

### 1. Runtime Guard Wrapper Contract

**Location:** [contracts/runtime-guard-wrapper/](contracts/runtime-guard-wrapper/)

A Soroban smart contract that wraps target contracts with runtime validation:

- **Guard Execution**: Pre/post-execution checks
- **Storage Validation**: Integrity verification
- **Metrics Collection**: Call tracking and performance data
- **Event Emission**: Guard status monitoring
- **Health Checks**: Continuous validation support

Key features:
```rust
// Initialize wrapper with target contract
init(env, wrapped_contract_address)

// Execute with guards
execute_guarded(env, function_name, args) -> Result<Val>

// Get validation stats
get_stats(env) -> (invariants_checked, call_count, failures)

// Health check for continuous monitoring
health_check(env) -> bool
```

### 2. CLI Deploy Command

**Location:** [tooling/sanctifier-cli/src/commands/deploy.rs](tooling/sanctifier-cli/src/commands/deploy.rs)

Integrated into Sanctifier CLI for easy deployment:

```bash
sanctifier deploy <PATH> [OPTIONS]

Options:
  --network <NETWORK>    Target network (testnet, futurenet, mainnet)
  --secret-key <KEY>     Soroban secret key
  --account-id <ID>      Account ID (optional)
  --validate             Run validation after deployment
  --output-format        Output format (text, json)
```

### 3. Bash Deployment Script

**Location:** [scripts/deploy-soroban-testnet.sh](scripts/deploy-soroban-testnet.sh)

Comprehensive shell script for production deployments:

```bash
./scripts/deploy-soroban-testnet.sh [OPTIONS]

Options:
  --network <NETWORK>    Target network (default: testnet)
  --no-validate         Skip post-deployment validation
  --no-continuous       Disable continuous validation loop
  --dry-run             Perform dry run without deployment
  --interval <SECONDS>  Validation interval (default: 300)
  --debug              Enable debug logging
```

Features:
- Automatic contract building
- WASM file discovery
- Deployment with retries
- Deployment manifest tracking
- Continuous validation loop
- Detailed logging

### 4. GitHub Actions Workflow

**Location:** [.github/workflows/soroban-deploy.yml](.github/workflows/soroban-deploy.yml)

Automated CI/CD pipeline that:
- Triggers on push to main or schedule
- Builds and validates WASM artifacts
- Deploys to testnet
- Runs continuous validation
- Generates deployment reports
- Posts status to GitHub

Triggers:
- **Push**: When runtime-guard-wrapper or deploy script changes
- **Schedule**: Every 6 hours for continuous validation
- **Manual**: Via Actions workflow dispatch

## 📊 Deployment Flow

```
┌─────────────────────────────────────────────────────┐
│ 1. Environment Validation                           │
│    - Check tools (cargo, soroban, jq, curl)        │
│    - Verify SOROBAN_SECRET_KEY                      │
│    - Validate network configuration                 │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ 2. Contract Building                                │
│    - Compile Rust to WASM                           │
│    - Optimize with release profile                  │
│    - Verify artifact generation                     │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ 3. Deployment                                       │
│    - Deploy WASM to Soroban testnet                 │
│    - Retrieve contract ID                           │
│    - Retry on failure (max 3 attempts)              │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ 4. Post-Deployment Validation                       │
│    - Invoke health_check() on contract              │
│    - Verify storage accessibility                   │
│    - Record deployment success                      │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ 5. Continuous Validation (Optional)                 │
│    - Loop every N seconds (default: 300)            │
│    - Call health_check() periodically               │
│    - Collect execution stats                        │
│    - Update deployment manifest                     │
└─────────────────────────────────────────────────────┘
```

## 🔐 Security & Secrets

### GitHub Secrets Setup

Add these secrets to your repository (`Settings > Secrets`):

```bash
# Required
SOROBAN_SECRET_KEY=SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Optional (derived if not set)
SOROBAN_ACCOUNT_ID=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

Using GitHub CLI:
```bash
gh secret set SOROBAN_SECRET_KEY --body "SBXXXXXXX..."
```

### Local Environment

```bash
# Copy template
cp .env.example .env.local

# Edit with credentials (NEVER commit!)
nano .env.local

# Load before running scripts
source .env.local
```

### Security Best Practices

✅ **DO:**
- Use GitHub Secrets for sensitive data
- Rotate credentials regularly
- Use separate keys for testnet/mainnet
- Review deployment logs
- Enable branch protection rules

❌ **DON'T:**
- Commit .env.local to repository
- Share secret keys in messages
- Hardcode credentials in scripts
- Store secrets in plain text

## 📝 Usage Examples

### Example 1: Deploy to Testnet

```bash
source .env.local
./scripts/deploy-soroban-testnet.sh --network testnet --validate
```

Output:
```
✓ Contract built: runtime-guard-wrapper
✓ Contract deployed: CXXXXX...
✓ Health check passed
✓ Contract validation passed
```

### Example 2: Dry Run

```bash
./scripts/deploy-soroban-testnet.sh --dry-run --debug
```

### Example 3: Custom Validation Interval

```bash
# Validate every 10 minutes instead of default 5
./scripts/deploy-soroban-testnet.sh --interval 600
```

### Example 4: Deploy without Continuous Validation

```bash
./scripts/deploy-soroban-testnet.sh --no-continuous --validate
```

### Example 5: CLI Deployment

```bash
sanctifier deploy contracts/runtime-guard-wrapper \
    --network testnet \
    --secret-key "$SOROBAN_SECRET_KEY" \
    --validate \
    --output-format json
```

## 📊 Deployment Artifacts

### Deployment Manifest

File: `.deployment-manifest.json`

```json
{
  "version": "1.0",
  "deployments": [
    {
      "contract_id": "CXXXXX...",
      "name": "runtime-guard-wrapper",
      "wasm_hash": "abc123...",
      "network": "testnet",
      "deployed_at": "2026-02-25T12:34:56Z",
      "last_validated": "2026-02-25T12:35:10Z",
      "status": "active"
    }
  ],
  "last_updated": "2026-02-25T12:35:10Z"
}
```

### Deployment Log

File: `.deployment.log`

Contains detailed execution logs for auditing:
```
[INFO] Deployment script started
[INFO] Network: testnet
[✓] Environment validated
[✓] Contract built: runtime-guard-wrapper
[✓] Contract deployed: CXXXXX...
[✓] Validation iteration #1
```

## ✅ Validation Checks

The continuous validation loop performs:

1. **Health Check**
   - Verifies wrapped contract is set
   - Checks metrics storage accessibility
   - Returns boolean status

2. **Stats Collection**
   - Invariants checked count
   - Call log entries
   - Guard failure count

3. **Storage Integrity**
   - Critical keys accessible
   - No corruption detected
   - State consistency maintained

4. **Event Monitoring**
   - Guard events emitted
   - Status tracking
   - Failure detection

## 🐛 Troubleshooting

### Issue: "SOROBAN_SECRET_KEY not found"

```bash
# Verify environment variable
echo $SOROBAN_SECRET_KEY

# Set if missing
export SOROBAN_SECRET_KEY="SBXXXXXXX..."

# Or load from .env.local
source .env.local
```

### Issue: "WASM file not found"

```bash
# Rebuild contracts
cargo build --release --target wasm32-unknown-unknown

# Verify output
ls -la target/wasm32-unknown-unknown/release/runtime_guard_wrapper.wasm
```

### Issue: "Contract deploy error"

```bash
# Check network connectivity
soroban network info --network testnet

# Verify account balance
soroban account balance --account $SOROBAN_ACCOUNT_ID --network testnet

# Check with dry-run
./scripts/deploy-soroban-testnet.sh --dry-run
```

### Issue: "Validation timeout"

```bash
# Increase validation interval
./scripts/deploy-soroban-testnet.sh --interval 600

# Or skip continuous validation
./scripts/deploy-soroban-testnet.sh --no-continuous
```

## 📚 Related Documentation

- [Full Deployment Guide](docs/soroban-deployment.md)
- [Runtime Guard Wrapper Contract](contracts/runtime-guard-wrapper/README.md)
- [Sanctifier CLI Documentation](tooling/sanctifier-cli/README.md)
- [GitHub Actions Workflow](https://docs.github.com/en/actions)
- [Soroban Documentation](https://soroban.stellar.org/docs)

## 🔄 Continuous Integration

### Scheduled Validation

The workflow runs automatically every 6 hours to validate deployed contracts:

```yaml
schedule:
  - cron: "0 */6 * * *"
```

### Manual Deployment

Trigger deployment manually:
1. Go to Actions tab
2. Select "Soroban Runtime Guard Deployment"
3. Click "Run workflow"
4. Select network (testnet/futurenet/mainnet)
5. Optionally enable dry-run

### Build Matrix (Future)

Planned support for:
- Multiple networks (testnet, futurenet, mainnet)
- Multiple contract versions
- Performance profiling
- Regression testing

## 📈 Performance Considerations

- Build time: ~30-45 seconds
- Deployment: ~10-15 seconds per contract
- Validation: ~5 seconds per check
- Continuous validation loop: Configurable, default 5 minutes

## 🎯 Next Steps

1. ✅ Set up environment variables
2. ✅ Add GitHub secrets
3. ✅ Test local deployment with dry-run
4. ✅ Deploy to testnet
5. ✅ Monitor validation logs
6. ✅ Review deployment manifest
7. ✅ Set up scheduled validation

## 📞 Support

For issues or questions:
- Check [Troubleshooting](#troubleshooting) section
- Review deployment logs in `.deployment.log`
- Check GitHub Actions run logs
- Consult [Full Documentation](docs/soroban-deployment.md)

---

**Last Updated:** February 25, 2026
**Version:** 1.0
**Status:** Production Ready
