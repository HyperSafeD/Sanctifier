# CI/CD Setup Guide for Soroban Runtime Guard Deployment

This guide covers the complete setup of continuous integration and deployment for the Sanctifier runtime guard wrapper contracts.

## Overview

The automation includes:
- **Local Testing**: Build and test contracts locally
- **GitHub Actions**: Automated deployment on push and schedule
- **Continuous Validation**: Periodic health checks and metrics collection
- **Artifact Management**: Deployment manifests and logs

## Prerequisites

### Local Development
- Rust 1.70+ (with wasm32-unknown-unknown target)
- Soroban CLI 20.0+
- Git
- Bash 4.0+

### GitHub Repository
- Actions enabled (default)
- Secrets management access
- Branch protection configured (recommended)

## Step 1: Local Setup

### 1.1 Install Tools

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Add WebAssembly target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli

# Verify
rustc --version
cargo --version
soroban --version
```

### 1.2 Create Local Environment

```bash
# Copy environment template
cp .env.example .env.local

# Edit with your credentials
nano .env.local
```

Edit `.env.local`:
```bash
SOROBAN_SECRET_KEY=SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
SOROBAN_NETWORK=testnet
DEPLOYMENT_NETWORK=testnet
```

### 1.3 Test Local Build

```bash
# Source environment
source .env.local

# Build contract
cargo build -p runtime-guard-wrapper \
    --release \
    --target wasm32-unknown-unknown

# Verify WASM
ls -lh target/wasm32-unknown-unknown/release/runtime_guard_wrapper.wasm

# Run tests
cargo test -p runtime-guard-wrapper
```

## Step 2: GitHub Secrets Configuration

### 2.1 Add Required Secrets

```bash
# Using GitHub CLI
gh auth login
gh secret set SOROBAN_SECRET_KEY --body "SBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# Optional: Account ID
gh secret set SOROBAN_ACCOUNT_ID --body "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
```

### 2.2 Via GitHub Web Interface

1. Go to your repository
2. Settings > Secrets and variables > Actions
3. Click "New repository secret"
4. Add each secret:
   - **Name:** SOROBAN_SECRET_KEY
   - **Value:** Your S-prefixed secret key

5. Optionally repeat for SOROBAN_ACCOUNT_ID

### 2.3 Verify Secrets

```bash
# List configured secrets
gh secret list

# Example output:
# SOROBAN_SECRET_KEY        Updated 2 minutes ago
# SOROBAN_ACCOUNT_ID        Updated 1 minute ago
```

## Step 3: Test Deployment

### 3.1 Local Dry Run

```bash
# Make script executable
chmod +x scripts/deploy-soroban-testnet.sh

# Run dry run
./scripts/deploy-soroban-testnet.sh --dry-run --debug

# Expected output:
# [INFO] Deployment script started
# [✓] Environment validated
# [DRY RUN] Contract would deploy: C...
```

### 3.2 Local Real Deployment (Optional)

```bash
# Deploy with continuous validation
source .env.local
./scripts/deploy-soroban-testnet.sh --network testnet

# Check results
cat .deployment-manifest.json | jq '.'
```

### 3.3 Test CLI Deployment

```bash
# Build CLI
cargo build -p sanctifier-cli --release

# Deploy via CLI
./target/release/sanctifier-cli deploy \
    contracts/runtime-guard-wrapper \
    --network testnet \
    --secret-key "$SOROBAN_SECRET_KEY" \
    --validate
```

## Step 4: GitHub Actions Workflow

### 4.1 Workflow Overview

The workflow file: `.github/workflows/soroban-deploy.yml`

**Triggers:**
- Push to main branch (if runtime-guard-wrapper changes)
- Schedule: Every 6 hours
- Manual: Via Actions tab

**Jobs:**
1. `build-and-deploy`: Build, deploy, validate
2. `continuous-validation`: Run health checks
3. `notification`: Generate reports

### 4.2 Workflow Permissions

Verify workflow permissions in your repository:

1. Settings > Actions > General
2. Workflow permissions:
   - ✓ Read and write permissions
   - ✓ Allow GitHub Actions to create and approve pull requests

### 4.3 Manual Workflow Dispatch

```bash
# Trigger workflow
gh workflow run soroban-deploy.yml \
    -f network=testnet \
    -f dry_run=false

# View workflow runs
gh run list --workflow soroban-deploy.yml
```

## Step 5: Monitoring & Verification

### 5.1 Check Workflow Status

```bash
# Get latest runs
gh run list --workflow soroban-deploy.yml --limit 5

# View detailed run
gh run view <RUN_ID>

# Get logs
gh run view <RUN_ID> > run-logs.txt
```

### 5.2 Check Deployment Artifacts

In GitHub Actions:
1. Go to Actions > Soroban Runtime Guard Deployment
2. Select workflow run
3. Download artifacts:
   - `deployment-manifest-<RUN_ID>`
   - `deployment-log-<RUN_ID>`

```bash
# Or verify locally
cat .deployment-manifest.json | jq '.'
head -50 .deployment.log
```

### 5.3 Validate Deployed Contract

```bash
# Get contract ID from manifest
CONTRACT_ID=$(jq -r '.deployments[0].contract_id' .deployment-manifest.json)

# Run health check
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network testnet \
    -- health_check

# Get stats
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network testnet \
    -- get_stats
```

## Step 6: Branch Protection (Recommended)

### 6.1 Configure Branch Rules

1. Go to Settings > Branches
2. Click "Add rule"
3. Branch name pattern: `main`
4. Configure protections:
   - ✓ Require status checks to pass
   - ✓ Require code reviews before merging
   - ✓ Require up-to-date branches
   - ✓ Require branches to be up to date before merging

### 6.2 Required Status Checks

Select these checks:
- ✓ build-and-deploy (Soroban Runtime Guard Deployment)
- ✓ Rust CI (existing)

## Step 7: Scheduled Validation

### 7.1 Understand Schedule

Current schedule in `soroban-deploy.yml`:
```yaml
schedule:
  - cron: "0 */6 * * *"  # Every 6 hours
```

This runs continuous validation every 6 hours.

### 7.2 Modify Schedule (Optional)

Edit `.github/workflows/soroban-deploy.yml`:

```yaml
schedule:
  - cron: "0 */4 * * *"  # Every 4 hours
  # or
  - cron: "0 9 * * *"    # Daily at 9 AM UTC
```

### 7.3 Monitor Scheduled Runs

```bash
# List recent scheduled runs
gh run list --workflow soroban-deploy.yml --limit 10

# Check latest scheduled run
gh run view $(gh run list --workflow soroban-deploy.yml -q) --log
```

## Step 8: Notifications (Optional)

### 8.1 Slack Integration

Create `.github/workflows/slack-notify.yml`:

```yaml
name: Slack Notification

on:
  workflow_run:
    workflows: ["Soroban Runtime Guard Deployment"]
    types: [completed]

jobs:
  notify:
    runs-on: ubuntu-latest
    steps:
      - name: Notify Slack
        uses: slackapi/slack-github-action@v1
        with:
          payload: |
            {
              "text": "Deployment ${{ job.status }}: ${{ github.repository }}"
            }
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK_URL }}
```

### 8.2 Email Notifications

Enable via GitHub Settings:
1. Settings > Notifications
2. Email notifications on workflow runs

## Troubleshooting

### Issue: "Secret SOROBAN_SECRET_KEY not available"

**Solution:**
```bash
# Verify secret exists
gh secret list

# If missing, add it
gh secret set SOROBAN_SECRET_KEY --body "SBXXXXXXX..."

# Re-run workflow
gh workflow run soroban-deploy.yml
```

### Issue: "Workflow failed: Soroban binary not found"

**Solution:**
The workflow automatically installs Soroban. If it fails:

1. Check network connectivity in workflow logs
2. Try manual dispatch with retry
3. File an issue if persists

### Issue: "Deployment successful but validation failed"

**Solution:**
1. Check contract health:
```bash
soroban contract invoke \
    --id $CONTRACT_ID \
    --network testnet \
    -- health_check
```

2. Check network status:
```bash
soroban network info --network testnet
```

3. Wait for contract to finalize on network

### Issue: "Cannot access secrets in local run"

**Solution:**
Secrets are GitHub-only. For local runs:
```bash
source .env.local
./scripts/deploy-soroban-testnet.sh --network testnet
```

## Best Practices

### ✅ DO:
- Use GitHub Secrets for all credentials
- Test with `--dry-run` first
- Review deployment logs
- Monitor continuous validation
- Rotate credentials regularly
- Use branch protection rules
- Archive deployment artifacts

### ❌ DON'T:
- Commit secrets to repository
- Disable branch protections
- Use same key for testnet/mainnet
- Ignore workflow failures
- Skip post-deployment validation
- Leave logs in plain text

## Maintenance

### Regular Tasks

```bash
# Weekly: Check deployment health
gh run list --workflow soroban-deploy.yml -L 7

# Monthly: Review deployment history
ls -la .deployment* | head -20

# Quarterly: Rotate credentials
gh secret set SOROBAN_SECRET_KEY --body "NEW_KEY_HERE"
```

### Archive Artifacts

```bash
# Backup deployment history
tar czf deployment-history-$(date +%Y%m%d).tar.gz .deployment*

# Move to archive
mv deployment-history-*.tar.gz archive/
```

## Related Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Soroban CLI Documentation](https://soroban.stellar.org/docs/tools/cli)
- [Secrets Management](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [Workflow Triggers](https://docs.github.com/en/actions/using-workflows/triggering-a-workflow)
- [Deployment Automation Guide](./SOROBAN_DEPLOYMENT.md)

## Next Steps

1. ✅ Complete local setup
2. ✅ Add GitHub secrets
3. ✅ Test dry run
4. ✅ Configure branch protection
5. ✅ Monitor first deployment
6. ✅ Set up notifications (optional)
7. ✅ Archive old artifacts regularly

---

**Last Updated:** February 25, 2026
**Version:** 1.0
