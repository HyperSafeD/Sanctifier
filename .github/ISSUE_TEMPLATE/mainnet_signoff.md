---
name: Mainnet Sign-Off Request
about: Final human-in-the-loop sign-off gate before tagging v1.0.0-mainnet (requires 2 reviewer approvals)
title: '[Sign-Off] Mainnet Readiness: v1.0.0-mainnet'
labels: sign-off, mainnet, release
assignees: ''

---

## 🚀 Mainnet Release Sign-Off Gate

This issue serves as the formal release approval gate for tagging `v1.0.0-mainnet`. 
Two named reviewer approvals are required before closing this issue and proceeding with the release.

---

### 1. 🛡️ Security Audit & Finding Verification
- [ ] All high / critical vulnerability detectors verified against test fixtures
- [ ] No unaddressed high / critical static analysis findings in default rules
- [ ] SMT invariant checks and Z3 formal verification suites passing green
- [ ] Contract security disclaimers and threat model updated

### 2. ⚡ Operations & Infrastructure
- [ ] CI pipeline completely passing on `main` (build, test, lint, coverage)
- [ ] Docker image build and registry publishing verified
- [ ] Binary releases and package publish dry-runs verified (`cargo`, `npm`, `homebrew`)
- [ ] Release secrets and environment tokens validated in repository settings

### 3. ⚖️ Compliance & Governance
- [ ] Licensing, security disclosures, and `SECURITY.md` verified
- [ ] `CHANGELOG.md` updated with all mainnet release entries
- [ ] Version string alignment across workspace crates, extensions, and packages
- [ ] Dependency tree audited for known vulnerabilities (`cargo deny`)

### 4. 🧪 Mainnet Fork & Real-World Testing
- [ ] Scheduled read-only mainnet fork CI job (`mainnet-fork-ci.yml`) passed without crash or hang
- [ ] Real-world mainnet contract corpus analyzed cleanly
- [ ] E2E integration and browser/VS Code extension tests passing

---

## 👥 Approvals (Minimum 2 Required)

> **Note:** Approvers must post an explicit sign-off comment (e.g. `Approved for mainnet release`) and check off their named approval slot below.

- [ ] **Primary Reviewer Approval**
  - **Approver Name / GitHub Handle:** `@`
  - **Date:** `YYYY-MM-DD`
  - **Status:** [ ] Approved

- [ ] **Secondary Reviewer Approval**
  - **Approver Name / GitHub Handle:** `@`
  - **Date:** `YYYY-MM-DD`
  - **Status:** [ ] Approved

---

### 📝 Notes & Exceptions
*Record any non-blocking warnings, accepted risks, or release notes here:*
- 
