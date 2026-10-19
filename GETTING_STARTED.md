# 🎉 Next Steps - Getting Started with Runtime Guard Deployment

Congratulations! You now have a complete automation platform for deploying runtime guard wrapper contracts to Soroban testnet. Here's what you can do next:

## 📋 Getting Started Checklist

### Immediate Actions (Today)

- [ ] **Read Quick Start**
  - File: [QUICK_START.md](QUICK_START.md)
  - Time: 5 minutes
  - Learn the basics and deploy your first contract

- [ ] **Set Up Environment**
  ```bash
  cp .env.example .env.local
  nano .env.local  # Add SOROBAN_SECRET_KEY
  ```

- [ ] **Test Dry Run**
  ```bash
  chmod +x scripts/deploy-soroban-testnet.sh
  ./scripts/deploy-soroban-testnet.sh --dry-run --debug
  ```

### Tomorrow - Deploy & Verify

- [ ] **Deploy to Testnet**
  ```bash
  source .env.local
  ./scripts/deploy-soroban-testnet.sh --network testnet
  ```

- [ ] **Verify Deployment**
  ```bash
  cat .deployment-manifest.json | jq '.'
  tail -50 .deployment.log
  ```

- [ ] **Check Contract Health**
  ```bash
  CONTRACT_ID=$(jq -r '.deployments[0].contract_id' .deployment-manifest.json)
  soroban contract invoke \
      --id "$CONTRACT_ID" \
      --network testnet \
      -- health_check
  ```

### This Week - CI/CD Setup (Optional)

- [ ] **Add GitHub Secret**
  ```bash
  gh secret set SOROBAN_SECRET_KEY --body "YOUR_KEY_HERE"
  ```

- [ ] **Test GitHub Actions**
  - Go to Actions tab
  - Select "Soroban Runtime Guard Deployment"
  - Click "Run workflow"
  - Monitor the run

- [ ] **Review Deployment Artifacts**
  - Download `deployment-manifest-*`
  - Download `deployment-log-*`
  - Verify successful deployment

### This Month - Production Setup

- [ ] **Review Full Documentation**
  - [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md) - Complete guide
  - [docs/ci-cd-setup.md](docs/ci-cd-setup.md) - CI/CD configuration
  - [ARCHITECTURE.md](ARCHITECTURE.md) - System design

- [ ] **Configure Branch Protection**
  - Settings > Branches > Add rule
  - Require status checks before merge
  - Enable deployment artifacts retention

- [ ] **Set Up Monitoring**
  - Review logs regularly: `tail -f .deployment.log`
  - Monitor GitHub Actions runs
  - Set up alerts/notifications (optional)

- [ ] **Migrate to Mainnet**
  - Read the [Mainnet Migration Guide](MAINNET_MIGRATION_GUIDE.md)
  - Consider separate keys per network
  - Test on futurenet if needed
  - Prepare mainnet deployment process

## 📚 Documentation by Use Case

### "I want to deploy now"
→ [QUICK_START.md](QUICK_START.md) (5 minutes)

### "I want to understand the system"
→ [ARCHITECTURE.md](ARCHITECTURE.md) + [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md)

### "I need to set up GitHub Actions"
→ [docs/ci-cd-setup.md](docs/ci-cd-setup.md)

### "I want to understand the contract"
→ [contracts/runtime-guard-wrapper/README.md](contracts/runtime-guard-wrapper/README.md)

### "I need the CLI tool"
→ `sanctifier deploy --help`

### "I'm having issues"
→ [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md#troubleshooting)

## 🚀 Usage Scenarios

### Scenario 1: Single Testnet Deployment
**Goal:** Deploy once, validate

```bash
# Setup (one time)
cp .env.example .env.local
# Edit .env.local

# Deploy
source .env.local
./scripts/deploy-soroban-testnet.sh --network testnet

# Done! Contract deployed and validated
```

### Scenario 2: Automated CI/CD
**Goal:** Auto-deploy on every push to main

```bash
# Setup (one time)
gh secret set SOROBAN_SECRET_KEY --body "YOUR_KEY"

# Now automatic:
# - Any push to main → GitHub Actions runs
# - Builds, deploys, validates automatically
# - Every 6 hours: continuous validation
```

### Scenario 3: Development & Testing
**Goal:** Frequent test deployments

```bash
# Use dry-run first
./scripts/deploy-soroban-testnet.sh --dry-run

# Deploy for testing
./scripts/deploy-soroban-testnet.sh --network testnet

# Iterate quickly
# Modify contract → test → redeploy
```

### Scenario 4: Production Validation
**Goal:** Continuous monitoring

```bash
# Deploy with monitoring
./scripts/deploy-soroban-testnet.sh --network testnet

# Script runs continuously
# Validates every 5 minutes
# Updates .deployment-manifest.json
```

## 🔧 Common Commands Reference

### Deployment
```bash
# Full deployment with validation
./scripts/deploy-soroban-testnet.sh --network testnet

# Dry run (no actual deployment)
./scripts/deploy-soroban-testnet.sh --dry-run

# Without continuous validation
./scripts/deploy-soroban-testnet.sh --no-continuous

# Custom validation interval
./scripts/deploy-soroban-testnet.sh --interval 600

# Debug mode
./scripts/deploy-soroban-testnet.sh --debug
```

### Verification
```bash
# Get deployed contract ID
jq -r '.deployments[0].contract_id' .deployment-manifest.json

# Health check
soroban contract invoke --id $CONTRACT_ID --network testnet -- health_check

# Get stats
soroban contract invoke --id $CONTRACT_ID --network testnet -- get_stats

# Watch logs
tail -f .deployment.log

# Latest deployment summary
head -20 .deployment.log
```

### GitHub Actions
```bash
# List recent runs
gh run list --workflow soroban-deploy.yml --limit 10

# View specific run
gh run view <RUN_ID>

# Download artifacts
gh run download <RUN_ID> -n deployment-manifest-<RUN_ID>
```

## 🎯 Success Criteria

Your setup is successful when:

✅ **Deployment Phase**
- [ ] `.deployment-manifest.json` created
- [ ] Contract ID recorded
- [ ] `.deployment.log` shows success messages

✅ **Validation Phase**
- [ ] `health_check()` returns `true`
- [ ] `get_stats()` shows execution data
- [ ] No guard failures reported

✅ **CI/CD Phase (if enabled)**
- [ ] GitHub Actions workflow completes
- [ ] Artifacts uploaded successfully
- [ ] Scheduled runs execute on time

✅ **Monitoring Phase**
- [ ] Continuous validation loop running
- [ ] Manifest updates every N seconds
- [ ] No errors in monitoring logs

## 🆘 Troubleshooting Quick Links

| Issue | Solution |
|-------|----------|
| SOROBAN_SECRET_KEY not found | [Setup Guide](docs/ci-cd-setup.md#environment-setup) |
| WASM file not found | [Build Guide](SOROBAN_DEPLOYMENT.md#build-contract) |
| Deployment failed | [Troubleshooting](SOROBAN_DEPLOYMENT.md#troubleshooting) |
| Validation timeout | [Options](QUICK_START.md#common-commands) |
| GitHub Actions not running | [CI/CD Setup](docs/ci-cd-setup.md#workflow-triggers) |

## 📞 Getting Help

1. **Check Documentation First**
   - Relevant guide from [Documentation by Use Case](#documentation-by-use-case)
   - Search for your error in [Troubleshooting](SOROBAN_DEPLOYMENT.md#troubleshooting)

2. **Review Examples**
   - [QUICK_START.md](QUICK_START.md) examples
   - [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md) use cases

3. **Check Logs**
   ```bash
   # Detailed logs
   cat .deployment.log | grep -i error
   
   # Recent lines
   tail -50 .deployment.log
   ```

4. **Test with Dry-Run**
   ```bash
   ./scripts/deploy-soroban-testnet.sh --dry-run --debug
   ```

5. **Verify Prerequisites**
   ```bash
   # Check tools
   cargo --version
   soroban --version
   jq --version
   ```

## 🌟 Advanced Topics (After Initial Setup)

Once you've deployed successfully, explore:

### 1. Custom Guard Rules
- Modify `GuardConfig` in the contract
- Add new invariant checks
- Extend metrics collection

### 2. Multi-Contract Deployment
- Deploy multiple contracts
- Wrap different contract types
- Coordinate deployments

### 3. Network Migration
- Test on futurenet
- Follow the [Mainnet Migration Guide](MAINNET_MIGRATION_GUIDE.md) to safely migrate to mainnet
- Manage multiple environments

### 4. Monitoring & Alerting
- Set up webhooks
- Create dashboards
- Configure notifications

### 5. Performance Tuning
- Optimize WASM size
- Reduce validation frequency
- Cache compilation

## 📊 Monitoring Dashboard (Future)

Planned features:
- [ ] Web-based deployment dashboard
- [ ] Real-time validation metrics
- [ ] Alert notifications
- [ ] Historical reporting
- [ ] Multi-network overview

## 🎓 Learning Path

**Level 1: Beginner** (Today)
→ Read QUICK_START.md → Deploy contract

**Level 2: Intermediate** (This week)
→ Set up GitHub Actions → Configure monitoring

**Level 3: Advanced** (This month)
→ Multi-network setup → Custom guards

## 🎯 Key Files to Know

| File | Purpose | Frequency |
|------|---------|-----------|
| QUICK_START.md | Get started quickly | Once |
| .env.local | Your credentials | Setup once |
| scripts/deploy-soroban-testnet.sh | Main deployment | Each deployment |
| .deployment-manifest.json | Deployment records | Auto-updated |
| .deployment.log | Execution logs | Auto-updated |
| .github/workflows/soroban-deploy.yml | CI/CD config | Edit as needed |
| ARCHITECTURE.md | System design | Reference |

## 💡 Pro Tips

✅ **Use Dry-Run First**
```bash
./scripts/deploy-soroban-testnet.sh --dry-run
```

✅ **Archive Old Manifests**
```bash
mv .deployment-manifest.json "manifest-$(date +%s).json"
```

✅ **Monitor in Real-Time**
```bash
watch -n 1 'tail -5 .deployment.log'
```

✅ **Check Before Pushing**
```bash
./scripts/deploy-soroban-testnet.sh --dry-run --debug
```

✅ **Keep Logs Organized**
```bash
tar czf "deployment-$(date +%Y%m%d).tar.gz" .deployment*
```

## 🚀 Launch Timeline

**Day 1**
- [ ] Read QUICK_START.md
- [ ] Set up environment
- [ ] Test dry-run

**Day 2**
- [ ] Deploy to testnet
- [ ] Verify health check
- [ ] Review logs

**Week 1**
- [ ] Set up GitHub Actions
- [ ] Test CI/CD
- [ ] Archive artifacts

**Week 2+**
- [ ] Monitor production
- [ ] Refine settings
- [ ] Plan next phases

## 📈 Success Metrics

Track these to measure success:

- Deployments: Number of successful deployments
- Uptime: Contract availability on testnet
- Health: Validation check pass rate
- Performance: Metrics collection efficiency
- Reliability: Error rates and recovery

## 🎊 Final Checklist

Before considering setup complete:

- [ ] Contract deployed successfully
- [ ] Health check passes
- [ ] Deployment manifest created
- [ ] Logs generated without errors
- [ ] GitHub Actions configured (if using)
- [ ] Continuous validation running
- [ ] Documentation reviewed
- [ ] Troubleshooting guide saved

---

## 🎯 You're Ready!

You now have everything needed to:
✅ Deploy runtime guard contracts  
✅ Automate validation  
✅ Set up CI/CD pipelines  
✅ Monitor deployments  
✅ Scale to multiple networks  

**Next Step:** Open [QUICK_START.md](QUICK_START.md) and deploy your first contract! 🚀

---

**Last Updated:** February 25, 2026  
**Status:** Ready for Production  
**Support:** Check SOROBAN_DEPLOYMENT.md for detailed help
