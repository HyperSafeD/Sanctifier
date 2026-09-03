# Mainnet Release Checklist

This checklist must be completed before any mainnet release tag can be cut.

## Mainnet Freeze Prerequisites

### Testing Gates (MUST PASS)

#### Coverage Gate (#1176)
- [ ] `sanctifier-core` coverage ≥ 90% (enforced in CI)
  - **Workflow**: `.github/workflows/e2e-coverage.yml`
  - **Config**: `codecov.yml` flag `sanctifier-core`
  - **Verification**: Check [Codecov dashboard](https://app.codecov.io/gh/HyperSafeD/Sanctifier)
  - **Status**: Run `bash scripts/measure-core-coverage.sh`

#### Mutation Testing Gate (#1178)
- [ ] Mutation kill rate ≥ 75% for `sanctifier-core`
  - **Workflow**: `.github/workflows/mutation-testing.yml`
  - **Tool**: `cargo-mutants`
  - **Verification**: Check latest workflow run artifacts
  - **Status**: `cd tooling/sanctifier-core && cargo mutants --no-shuffle`

#### Extended Fuzz Campaign (#1179)
- [ ] 24h+ fuzz campaign completed with zero unresolved crashes
  - **Script**: `bash scripts/run-extended-fuzz.sh 24`
  - **Documentation**: `docs/FUZZ_CAMPAIGN_[DATE].md`
  - **Corpus**: New inputs committed to `tooling/sanctifier-core/fuzz/corpus/`
  - **Verification**: Review campaign report document

#### E2E Regression Suite (#1177)
- [ ] Full deploy flow E2E test passing (upload → scan → deploy)
  - **Test**: `e2e/tests/full-deploy-flow.spec.ts`
  - **Workflow**: `.github/workflows/e2e-coverage.yml`
  - **Verification**: `cd e2e && npx playwright test full-deploy-flow.spec.ts`

---

## Code Quality

### Static Analysis
- [ ] All Clippy warnings resolved (`cargo clippy -- -D warnings`)
- [ ] No security advisories in dependencies (`cargo audit`)
- [ ] Formatting verified (`cargo fmt --check`)

### Documentation
- [ ] CHANGELOG.md updated with all changes
- [ ] API documentation complete (`cargo doc --no-deps --document-private-items`)
- [ ] Breaking changes clearly documented
- [ ] Migration guide provided (if applicable)

---

## Security Review

- [ ] Security audit completed (if major release)
- [ ] Dependency audit passed (`cargo audit`)
- [ ] No known high/critical vulnerabilities
- [ ] Secrets and keys rotated (if applicable)

---

## Deployment Verification

### Testnet Validation
- [ ] Full deployment tested on testnet
- [ ] Contract interactions verified
- [ ] Monitoring and alerting configured
- [ ] Rollback procedure documented and tested

### Mainnet Safety Gates
- [ ] `--confirm-mainnet` flag implemented and tested
- [ ] Passphrase confirmation working
- [ ] Rate limiting in place (if applicable)
- [ ] Circuit breakers configured (if applicable)

---

## Release Process

### Pre-Release
- [ ] Version bumped in `Cargo.toml`
- [ ] Git tag created: `git tag -a v[VERSION] -m "Release v[VERSION]"`
- [ ] Release notes drafted in GitHub Releases

### Post-Release
- [ ] Release published on GitHub
- [ ] Crates.io publication verified (if applicable)
- [ ] Docker images built and pushed (if applicable)
- [ ] Documentation site updated
- [ ] Community announcement posted

---

## Sign-Off

**Release Manager**: ___________________________  Date: __________

**Security Lead**: ___________________________  Date: __________

**Technical Lead**: ___________________________  Date: __________

---

## Notes

Use this space to document any exceptions, waivers, or additional context:

```
[Add notes here]
```
