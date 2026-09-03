# [EPIC] Mainnet Launch Readiness — Tracking Issue for All A1–A7 Workstreams

**Issue:** #1252  
**Milestone:** Mainnet Launch Readiness  
**Status:** In Progress  
**Last Updated:** 2026-07-27

---

## Overview

This epic aggregates all 78 issues (#1112-#1189) in the **Mainnet Launch Readiness** milestone, providing a single rollup view of mainnet-launch progress across seven workstreams (A1-A7). Stakeholders can monitor completion status at a glance without scanning the full milestone issue list.

This epic serves as:
- The central dashboard for mainnet readiness
- The reference point for automated release-gate checks (#1189)
- The checklist used in the sign-off template (#1185)

---

## Workstream Breakdown

### A1: Security Hardening (14 issues)

Cryptographic audits, formal verification, threat modeling, and security infrastructure.

- [ ] #1112: Commission external smart-contract security audit before mainnet launch
- [ ] #1113: Internal red-team review of all contract examples for mainnet-exposure risk
- [ ] #1114: Full formal-verification (Kani+Z3) pass required on every contract before freeze
- [ ] #1115: Publish mainnet launch security checklist as SECURITY.md addendum
- [ ] #1116: Stand up a bug bounty program (Immunefi/HackenProof listing)
- [ ] #1117: Dependency supply-chain audit + SBOM generation (cargo-deny, npm audit, syft)
- [ ] #1118: Audit and rotate all dev/test secrets before mainnet cutover
- [ ] #1119: Require admin multisig + timelock for privileged contract functions
- [ ] #1120: Document key-ceremony / cold-storage procedure for mainnet deployer keys
- [ ] #1121: Add automated secret-scanning gate (gitleaks) to CI, block on detection
- [ ] #1122: Threat-model the hosted API/dashboard for mainnet traffic patterns
- [ ] #1123: Commission penetration test of frontend dashboard and public API
- [ ] #1173: Add mainnet quickstart documentation to README.md and GETTING_STARTED.md
- [ ] #1174: Run full documentation link/code-sample audit before mainnet freeze

---

### A2: Contract Hardening (12 issues)

Hardening contract examples, runtime guards, resource limits, and upgrade governance.

- [ ] #1124: Add network-passphrase guard preventing accidental cross-network deploys
- [ ] #1125: Audit all example/reference contracts for mainnet-safe default configuration
- [ ] #1126: Full circuit-breaker/pause-mechanism audit across all guard contracts
- [ ] #1127: Enforce upgrade-governance timelock in runtime-guard-wrapper
- [ ] #1128: Build gas/resource-fee benchmark suite validated against Soroban mainnet limits
- [ ] #1129: Add static rule flagging TODO/unimplemented/test-only code paths pre-deploy
- [ ] #1130: Verify SEP-41 compliance suite against live mainnet fee model
- [ ] #1131: Add resource-metered reentrancy-guard stress test for mainnet-scale calls
- [ ] #1132: Publish immutable-vs-upgradeable decision matrix per contract (ADR)
- [ ] #1175: Add video walkthrough demonstrating full mainnet workflow (extend existing series)
- [ ] #1181: Provide worked examples for mainnet canary deployments
- [ ] #1184: Add CI budget gate on WASM bundle size before mainnet release

---

### A3: Deployment Operations (10 issues)

Deployment tooling, runbooks, rollback procedures, and release management.

- [ ] #1133: Require explicit `--confirm-mainnet` flag in deploy tooling to block accidental mainnet targeting
- [ ] #1134: Write end-to-end mainnet deployment runbook with rollback plan
- [ ] #1135: Add mainnet dry-run mode to `scripts/deploy.sh`
- [ ] #1136: Extend `soroban-deploy.yml` with a manual-approval-gated mainnet target
- [ ] #1137: Document rollback/circuit-breaker procedure for failed mainnet deploys
- [ ] #1138: Define semver + release policy for mainnet-stable CLI/core (v1.0.0)
- [ ] #1139: Automate CHANGELOG generation for mainnet release notes
- [ ] #1140: Add mainnet sign-off addendum to RELEASE_CHECKLIST.md
- [ ] #1141: Document canary/staged-rollout strategy for the frontend dashboard
- [ ] #1142: Add a feature-flag system to gate unfinished features in production

---

### A4: Reliability & Monitoring (16 issues)

Production infrastructure, monitoring, error tracking, load testing, and disaster recovery.

- [ ] #1143: Harden Dockerfile for production (distroless/minimal base image)
- [ ] #1144: Add container image vulnerability scanning (Trivy) to CI
- [ ] #1145: Pin and audit every Cargo/npm lockfile before mainnet freeze
- [ ] #1146: Add Soroban RPC provider redundancy/failover (multi-provider fallback)
- [ ] #1147: Stand up uptime monitoring + public status page for dashboard/API
- [ ] #1148: Integrate error tracking (Sentry) across frontend and API
- [ ] #1149: Add structured logging + log aggregation for CLI/API/frontend
- [ ] #1150: Build Grafana/Prometheus dashboards for API latency and error rate
- [ ] #1151: Extend `testnet-monitor.yml` to also monitor mainnet contract health
- [ ] #1152: Add alerting rules for contract-pause events and failed verifications
- [ ] #1153: Define and document SLOs/SLA for the hosted scanning service
- [ ] #1154: Build load/stress test suite for frontend+API at mainnet-scale traffic
- [ ] #1155: Add chaos-testing scenarios: RPC outage, timeout, malformed response
- [ ] #1156: Add automated backup + disaster-recovery plan for vuln DB and user data
- [ ] #1157: Set up on-call rotation and incident-response playbook
- [ ] #1188: Verify CLI exit codes and error taxonomy are stable/documented for CI consumers before freeze

---

### A5: Compliance & Legal (8 issues)

Terms of service, privacy policy, GDPR compliance, licensing, and legal review.

- [ ] #1158: Draft Terms of Service and Privacy Policy for hosted dashboard
- [ ] #1159: Review telemetry opt-in/out flow for GDPR compliance before mainnet
- [ ] #1160: Document data-retention policy for uploaded contracts and scan results
- [ ] #1161: Finalize license-compliance report (cargo-deny + license-checker) across all packages
- [ ] #1162: Add data-processing agreement template for enterprise users
- [ ] #1163: Finalize SECURITY.md disclosure policy for mainnet-era vulnerabilities
- [ ] #1164: Add cookie-consent banner if analytics cookies are used on the dashboard
- [ ] #1165: Legal review of bundled contract examples for licensing conflicts

---

### A6: Distribution & Packaging (5 issues)

Release artifacts, package managers, browser extensions, and launch announcements.

- [ ] #1166: Final QA of Homebrew/Scoop/Winget/npm packages against mainnet-stable version
- [ ] #1167: Submit browser extension to Chrome Web Store and Firefox Add-ons
- [ ] #1168: Final WCAG 2.1 AA accessibility audit of the frontend dashboard
- [ ] #1169: Final mobile-responsiveness pass on the frontend dashboard
- [ ] #1170: Publish launch-announcement checklist (blog, socials, Stellar community)

---

### A7: Testing & Validation (3 issues)

Migration guides, environment indicators, and automated release gates.

- [ ] #1171: Write mainnet migration guide for existing testnet users
- [ ] #1172: Add a persistent "Mainnet/Testnet" environment indicator across CLI, dashboard, reports
- [ ] #1189: Add a pre-mainnet "cold checklist" GitHub Action that blocks release tag push until all gate issues close

---

## Progress Summary

**Total Issues:** 68 (excluding this tracking epic #1252)  
**Completed:** 0  
**In Progress:** 0  
**Blocked:** 0  
**Remaining:** 68

**Workstream Status:**
- A1 Security Hardening: 0/14 (0%)
- A2 Contract Hardening: 0/12 (0%)
- A3 Deployment Operations: 0/10 (0%)
- A4 Reliability & Monitoring: 0/16 (0%)
- A5 Compliance & Legal: 0/8 (0%)
- A6 Distribution & Packaging: 0/5 (0%)
- A7 Testing & Validation: 0/3 (0%)

---

## Critical Path

The following issues are on the critical path and must be completed before mainnet launch:

**P0 Blockers:**
- #1112: External security audit
- #1114: Full formal verification pass
- #1121: Secret scanning gate
- #1133: `--confirm-mainnet` safety flag
- #1189: Release-gate GitHub Action

**P1 High Priority:**
- #1113: Red-team review
- #1116: Bug bounty program
- #1118: Secret rotation
- #1124: Network passphrase guard
- #1140: Mainnet sign-off checklist
- #1147: Uptime monitoring
- #1158: Terms of Service / Privacy Policy

---

## Dependencies

### Upstream Dependencies
- [ ] Soroban mainnet launch (external)
- [ ] Mainnet RPC provider availability (external)

### Internal Dependencies
- #1185 (mainnet sign-off template) ← depends on this epic
- #1189 (release-gate check) ← depends on this epic
- #1171 (migration guide) ← depends on #1124, #1133 (safety flags)

---

## Maintenance Protocol

This epic is maintained as follows:

1. **Checklist Updates**: Manually check boxes as issues close (automated sync TBD)
2. **Progress Summary**: Update weekly based on milestone closure rate
3. **Critical Path**: Update as priorities shift or blockers emerge
4. **Status**: Mark complete when all A1-A7 issues are closed

---

## Automation Status

**Current:** Manual checklist maintenance  
**Planned:** GitHub Action to auto-sync checkbox state with actual issue closure (future enhancement)

---

## Sign-Off Template Reference

When all issues are closed, use the mainnet sign-off template (#1185) to obtain stakeholder approval before tagging the mainnet-ready release.

**Sign-Off Criteria:**
- [ ] All A1-A7 checklist items completed
- [ ] External audit passed
- [ ] Bug bounty program live
- [ ] All P0/P1 issues closed
- [ ] Release gate (#1189) passing

---

## Related Documentation

- [MAINNET_READINESS_PLAN.md](./MAINNET_READINESS_PLAN.md) - Implementation guide for #1173, #1174, #1175, #1181
- [RELEASE_CHECKLIST.md](./docs/RELEASE_CHECKLIST.md) - General release checklist
- [SECURITY.md](./SECURITY.md) - Security policy
- `.github/ISSUE_TEMPLATE/mainnet_signoff.md` - Sign-off template (#1185)
- [docs/PROGRAM_LABELS.md](./docs/PROGRAM_LABELS.md) - Milestone and label taxonomy

---

## Notes

- This epic does NOT include ZK Integration milestone issues (#1190-#1261) — those are tracked separately
- Issue numbers #1173-#1189 have gaps; some numbers may be reserved for future use or closed/deleted
- Progress percentages are updated manually; automation coming in future iteration
- For questions or updates, comment on #1252

---

**Last Sync:** 2026-07-27  
**Next Review:** Weekly until mainnet launch
