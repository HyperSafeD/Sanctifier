# 📑 Deployment Automation Documentation Index

**Quick navigation for all deployment automation documentation.**

---

## 🚀 Getting Started (Start Here!)

### For Beginners

1. **[QUICK_START.md](QUICK_START.md)** - 5-minute setup guide
   - Minimal setup
   - Deploy your first contract
   - Verify success

2. **[GETTING_STARTED.md](GETTING_STARTED.md)** - Complete onboarding
   - Checklist for setup
   - Common commands
   - Success criteria

### For Decision Makers

- **[COMPLETION_REPORT.md](COMPLETION_REPORT.md)** - What was delivered
- **[IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)** - What's included

### Contributor & Community Policies

- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contribution workflow and PR process
- **[Code of Conduct](.github/CODE_OF_CONDUCT.md)** - Community behavior expectations

---

## 📚 Comprehensive Guides

### Main Deployment Guide

**[SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md)** - Complete deployment documentation

- Overview of the system
- Prerequisites and setup
- Environment configuration
- Local deployment instructions
- CI/CD pipeline overview
- Continuous validation details
- Troubleshooting guide
- Deployment verification
- Performance optimization
- Best practices

### CI/CD Setup

**[docs/ci-cd-setup.md](docs/ci-cd-setup.md)** - GitHub Actions configuration

- Step-by-step CI/CD setup
- GitHub Secrets configuration
- Workflow triggers
- Deployment verification
- Branch protection rules
- Monitoring and notifications
- Troubleshooting
- Best practices for production

### Contracts Fuzz Harness CI

**[docs/contracts-fuzz.md](docs/contracts-fuzz.md)** - Fuzz harness coverage

- Bolero in-process property tests (every PR)
- cargo-fuzz / cargo-bolero coverage-guided harness (nightly)
- Cross-contract message wire format stability
- Local reproduction recipes

### Technical Architecture

**[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and components

- Component overview
- Data flow diagrams
- Deployment flow
- Secret management
- State management
- Metrics collection
- Deployment lifecycle

---

## 🛠️ Component Documentation

### Contract ABI / Interface Reference
**[docs/contract-interfaces.md](docs/contract-interfaces.md)** - Public ABI for all contracts in `contracts/*`
- Function signatures and descriptions for every contract
- Error code tables
- Machine-readable JSON at `docs/generated/contract-interfaces.json`
- Regenerate with `make contract-docs`; CI enforces freshness via `make contract-docs-check`

### Contract Security Disclaimers
**[docs/contract-security-disclaimers.md](docs/contract-security-disclaimers.md)** - Security disclaimer framework for contracts
- Security level classification (Critical, High, Medium, Low)
- Disclaimer categories (Audit, Usage, Upgrade, Emergency)
- Implementation guide and usage examples
- Testing and validation procedures
- Security best practices and monitoring
- Integration examples for contract developers

### Runtime Guard Wrapper Contract

**[contracts/runtime-guard-wrapper/README.md](contracts/runtime-guard-wrapper/README.md)**

- Contract architecture
- Public functions
- Internal guards
- Storage layout
- Configuration
- Events
- Testing
- Deployment
- Performance
- Security considerations
- Integration examples
- Troubleshooting

### Sanctifier CLI Deploy Command

**Location:** `tooling/sanctifier-cli/src/commands/deploy.rs`

- Integrated into sanctifier CLI
- Single-command deployment
- Automatic validation

### Rule Authoring Guide

**[docs/rule-authoring-guide.md](docs/rule-authoring-guide.md)**

- Rule anatomy and matcher types
- Built-in rules (`S001`–`S012`) overview
- Custom rule authoring (TOML inline + YAML files)
- `.sanctify.toml` configuration reference
- Testing fixtures and CI validation
- Severity guidelines and output stability
- Contribution checklist for rule PRs

### WASM Module Versioning & Input Validation

**[docs/wasm-versioning-alignment.md](docs/wasm-versioning-alignment.md)** - WASM module hardening

- Versioning strategy
- Input validation
- API changes
- Migration guide
- Performance considerations
- Testing guide
- [CSP Security Hardening](docs/wasm-security-hardening.md)

### Bash Deployment Script

**Location:** `scripts/deploy-soroban-testnet.sh`

- Production-ready automation
- Comprehensive configuration options
- Detailed logging
- Continuous validation support

### GitHub Actions Workflow

**Location:** `.github/workflows/soroban-deploy.yml`

- Automated CI/CD
- Multiple job types
- Scheduled validation
- Artifact management

### Sanctifier Scan GitHub Action

**Location:** `action.yml`

- Composite action for running `sanctifier analyze` in CI
- Support matrix: `docs/github-action-support-matrix.md` (includes debug logging mode)
- Threat model notes: `docs/github-action-threat-model.md`

### Docs and Specs Maintenance

- Docs/specs CI coverage: [docs/docs-specs-ci-coverage.md](docs/docs-specs-ci-coverage.md)
- Troubleshooting guide boundaries: [docs/troubleshooting-guide.md](docs/troubleshooting-guide.md)
- Support/compatibility matrix: [docs/github-action-support-matrix.md](docs/github-action-support-matrix.md)
- API reference generation: [docs/api-reference-generation.md](docs/api-reference-generation.md)

### Release Artifacts (data/ + schemas/)

- Threat model and operational guarantees: [docs/release-artifacts-threat-model.md](docs/release-artifacts-threat-model.md)
- How to verify a downloaded artifact: [docs/provenance-verification.md](docs/provenance-verification.md)
- Canonical artifact list: [data/release-manifest.json](data/release-manifest.json)

---

## 📚 Documentation Map by Use Case

### "I want to deploy now"

→ **[QUICK_START.md](QUICK_START.md)** (5 min)  
→ Run: `./scripts/deploy-soroban-testnet.sh --network testnet`

### "I want to understand the system"

→ **[ARCHITECTURE.md](ARCHITECTURE.md)** (15 min)  
→ **[IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)** (10 min)

### "I need to set up CI/CD"

→ **[docs/ci-cd-setup.md](docs/ci-cd-setup.md)** (20 min)  
→ Add GitHub secrets  
→ Push to main

### "I need complete details"

→ **[SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md)** (45 min)  
→ Complete reference

### "Something is broken"

→ **[SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md#troubleshooting)** - Troubleshooting  
→ Check logs: `tail -f .deployment.log`

### "I'm planning deployment"

→ **[COMPLETION_REPORT.md](COMPLETION_REPORT.md)**  
→ Check feature list and statistics

### "I need production guidelines"

→ **[docs/ci-cd-setup.md](docs/ci-cd-setup.md)** - Security section  
→ **[SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md)** - Best practices

---

## 🎯 Documentation by Topic

### Setup & Configuration

| Document                                       | Time   | Topics            |
| ---------------------------------------------- | ------ | ----------------- |
| [QUICK_START.md](QUICK_START.md)               | 5 min  | Quick setup       |
| [GETTING_STARTED.md](GETTING_STARTED.md)       | 10 min | Full checklist    |
| [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md) | 30 min | All setup options |
| [docs/ci-cd-setup.md](docs/ci-cd-setup.md)     | 20 min | GitHub setup      |

### Understanding the System

| Document                                               | Time   | Topics           |
| ------------------------------------------------------ | ------ | ---------------- |
| [ARCHITECTURE.md](ARCHITECTURE.md)                     | 15 min | System design    |
| [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) | 10 min | What's included  |
| [COMPLETION_REPORT.md](COMPLETION_REPORT.md)           | 5 min  | Deliverables     |
| Contract README                                        | 20 min | Contract details |
| [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md) | 15 min | Rule authoring |

### Deployment Operations

| Document                                       | Time   | Topics           |
| ---------------------------------------------- | ------ | ---------------- |
| [QUICK_START.md](QUICK_START.md)               | 5 min  | First deployment |
| [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md) | 30 min | All operations   |
| `.github/workflows/soroban-deploy.yml`         | 10 min | CI/CD workflow   |

### Troubleshooting & Support

| Document                                                                                         | Time   | Topics        |
| ------------------------------------------------------------------------------------------------ | ------ | ------------- |
| [docs/troubleshooting-guide.md](docs/troubleshooting-guide.md)                                   | 5 min  | Owner map     |
| [SOROBAN_DEPLOYMENT.md#troubleshooting](SOROBAN_DEPLOYMENT.md#troubleshooting)                   | 15 min | Common issues |
| [docs/ci-cd-setup.md#troubleshooting](docs/ci-cd-setup.md#troubleshooting)                       | 10 min | CI/CD issues  |
| [GETTING_STARTED.md#troubleshooting-quick-links](GETTING_STARTED.md#troubleshooting-quick-links) | 5 min  | Quick answers |

---

## 📋 Quick Reference

### Common Commands

**Deploy**

```bash
./scripts/deploy-soroban-testnet.sh --network testnet
```

See: [QUICK_START.md - Common Commands](QUICK_START.md#-common-commands)

**Verify**

```bash
cat .deployment-manifest.json | jq '.'
```

See: [SOROBAN_DEPLOYMENT.md - Verification](SOROBAN_DEPLOYMENT.md#deployment-verification)

**Monitor**

```bash
tail -f .deployment.log
```

See: [QUICK_START.md - Verification](QUICK_START.md#-check-results-1-min)

---

## 🔐 Security & Best Practices

**Secrets Management**

- GitHub Secrets: [docs/ci-cd-setup.md](docs/ci-cd-setup.md#github-secrets-configuration)
- Local Setup: [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md#secret-management)
- Best Practices: [SOROBAN_DEPLOYMENT.md - Security](SOROBAN_DEPLOYMENT.md#security-best-practices)

**GitHub Action Hardening**

- Support Matrix: [docs/github-action-support-matrix.md](docs/github-action-support-matrix.md)
- Threat Model Notes: [docs/github-action-threat-model.md](docs/github-action-threat-model.md)
- Docs/specs CI coverage: [docs/docs-specs-ci-coverage.md](docs/docs-specs-ci-coverage.md)
- Action unit test fixtures: [tests/action/fixtures](tests/action/fixtures)
- Vulnerability DB format and validation: [docs/vulnerability-database-format.md](docs/vulnerability-database-format.md)
- Data + schemas performance budgets/benchmarks: [docs/data-schemas-performance.md](docs/data-schemas-performance.md)
- Docs site navigation performance budgets/benchmarks: [docs/docs-navigation-performance.md](docs/docs-navigation-performance.md)
- WASM CSP Security Hardening: [docs/wasm-security-hardening.md](docs/wasm-security-hardening.md)

**Production Setup**

- Branch Protection: [docs/ci-cd-setup.md](docs/ci-cd-setup.md#branch-protection-recommended)
- Network Security: [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md#network-security)
- Audit Trails: [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md#audit-trail)

---

## 🎓 Learning Path

### Day 1: Getting Started

1. Read [QUICK_START.md](QUICK_START.md) (5 min)
2. Set up environment (5 min)
3. Run dry-run (2 min)
4. Deploy contract (5 min)
5. Verify deployment (3 min)
   **Total: 20 minutes** ✅

### Week 1: Production Setup

1. Read [ARCHITECTURE.md](ARCHITECTURE.md) (15 min)
2. Review [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md) (30 min)
3. Set up [GitHub Actions](docs/ci-cd-setup.md) (20 min)
4. Configure branch protection (10 min)
5. Test CI/CD (15 min)
   **Total: 90 minutes** ✅

### Month 1: Advanced Topics

1. Extend Runtime Guardians
2. Multi-network deployment
3. Custom monitoring
4. Performance tuning
5. Disaster recovery planning

---

## 📞 Checklists

### Pre-Deployment

- [ ] Read [QUICK_START.md](QUICK_START.md)
- [ ] Copy `.env.example` to `.env.local`
- [ ] Add `SOROBAN_SECRET_KEY`
- [ ] Run `--dry-run`
- [ ] Ready to deploy

### Post-Deployment

- [ ] Check `.deployment-manifest.json`
- [ ] Call `health_check()`
- [ ] Review `.deployment.log`
- [ ] Get `get_stats()`
- [ ] Deployment successful!

### CI/CD Setup

- [ ] Add GitHub secrets
- [ ] Review workflow file
- [ ] Test manual dispatch
- [ ] Verify automatic triggers
- [ ] Monitor first run
- [ ] CI/CD operational!

---

## 🔗 External Resources

### Soroban Documentation

- [Soroban Docs](https://soroban.stellar.org/docs)
- [Soroban CLI Reference](https://soroban.stellar.org/docs/tools/cli)
- [Stellar Networks](https://soroban.stellar.org/docs/networks)

### GitHub Resources

- [GitHub Actions](https://docs.github.com/en/actions)
- [Secrets Management](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)

### Related Sanctifier Docs

- [Getting Started](docs/getting-started.md)
- [Kani Integration](docs/kani-integration.md)
- [Benchmark Methodology](specs/BENCHMARK_METHODOLOGY.md)
- [Supply-Chain Provenance Verification](docs/provenance-verification.md)
- [Architecture Decisions](docs/adr/)

---

## 📊 Documentation Statistics

| Document                  | Pages  | Read Time   | Focus              |
| ------------------------- | ------ | ----------- | ------------------ |
| QUICK_START.md            | 4      | 5 min       | Getting started    |
| GETTING_STARTED.md        | 8      | 10 min      | Planning           |
| SOROBAN_DEPLOYMENT.md     | 12     | 30 min      | Complete reference |
| docs/ci-cd-setup.md       | 10     | 20 min      | GitHub Actions     |
| ARCHITECTURE.md           | 10     | 15 min      | System design      |
| IMPLEMENTATION_SUMMARY.md | 8      | 10 min      | Deliverables       |
| COMPLETION_REPORT.md      | 6      | 5 min       | Summary            |
| Contract README           | 11     | 20 min      | Technical          |
| **Total**                 | **69** | **115 min** | **Complete**       |

---

## 🎯 Next Steps

1. **Start Here:** [QUICK_START.md](QUICK_START.md)
2. **Then:** Set up environment and deploy
3. **Next:** Review [SOROBAN_DEPLOYMENT.md](SOROBAN_DEPLOYMENT.md)
4. **Finally:** Set up [GitHub Actions](docs/ci-cd-setup.md)

---

## ✅ What You Get

✅ **Complete Automation**

- CLI tool
- Bash script
- GitHub Actions workflow

✅ **Comprehensive Documentation**

- 7 guides
- Code examples
- Troubleshooting

✅ **Production Ready**

- Error handling
- Security hardened
- Fully tested

✅ **Easy to Use**

- 5-minute setup
- Single command deployment
- Automatic validation

---

**Last Updated:** February 25, 2026  
**Version:** 1.0  
**Status:** Production Ready

🎉 Start with [QUICK_START.md](QUICK_START.md) and deploy your first contract in 5 minutes!

- [Schema publish pipeline & Typed Bindings](docs/schema-pipeline-bindings-notes.md)
