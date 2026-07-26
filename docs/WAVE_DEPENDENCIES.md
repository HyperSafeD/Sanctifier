# Cross-Milestone Dependencies: ZK Rules ↔ Mainnet-Freeze Testing Gate

> **Issue #1257** — Cross-team kickoff doc.
> Linked from epic #1252 (Mainnet Launch Readiness) and epic #1253 (ZK Integration).

This document makes the cross-cutting dependencies between the two Next Wave
milestones explicit so no contributor accidentally ships past a blocking
dependency. It supplements (but does not replace) the individual issue
checklists.

---

## Milestone Overview

| Milestone | Epic | Focus |
|-----------|------|-------|
| **Mainnet Launch Readiness** | #1252 | Audit, freeze testing, performance gates, deployment hardening |
| **ZK Integration** | #1253 | Z-rule namespace, ZK rules Z001–Z099, ZK specialist review, feature flag removal |

Both milestones may be worked in parallel by different contributors. The
dependencies below are **hard gates** — the downstream work must not merge
before the upstream item lands.

---

## Hard Dependencies

### 1. Performance-regression gate before Z-rules land

**Upstream:** #1182 — Performance regression CI gate (Mainnet milestone)
**Downstream:** #1197–#1210 — Z-rules Z001–Z014 (ZK milestone)

**Rationale:** Z-rules add new analysis passes that run on every `sanctify`
invocation. The regression gate in #1182 establishes a baseline latency budget
and CI check. If Z-rules land first, the gate has no baseline to compare
against and will pass trivially even if the rules cause a 10× slowdown.

**Action:** Do not merge any of #1197–#1210 until #1182's CI gate is green on
`main`.

---

### 2. ZK specialist review gates ZK feature-flag removal — independent of main audit

**Upstream:** #1251 — ZK specialist security review
**Downstream:** #1142 — Remove `zk_features` feature flag (ZK milestone)

**Rationale:** The ZK verifier integration (#1142) is behind a feature flag
pending expert review. The main mainnet security audit (#1112) covers the
overall contract surface but is **not** a substitute for the ZK-specific
review (#1251) because ZK circuit soundness requires specialist expertise. The
flag must not be removed until #1251 delivers a clean report, regardless of
whether #1112 is complete.

**Action:** #1142 is blocked on #1251. Do not merge #1142 until #1251 is
resolved, even if #1112 closes first.

---

### 3. Namespace/severity ADR before any Z-rule is merged

**Upstream:** #1192 — `Z0xx` namespace and severity taxonomy ADR
**Downstream:** #1197–#1210, #1230 (all Z-rules and rule-registry wiring)

**Rationale:** The ADR defines the finding-code numbering scheme and SARIF
severity mapping that every Z-rule must conform to. Rules merged before the
ADR creates a patch-up burden and may ship inconsistent severity ratings into
production SARIF output.

**Action:** #1192 must merge before any Z-rule PR (#1197–#1210) or the
rule-registry wiring (#1230) is reviewed.

---

### 4. Labels and milestones configured before triage automation activates

**Upstream:** #1255 — Label consistency pass; #1256 — Milestone setup
**Downstream:** #1260 — Project-board automation

**Rationale:** The auto-triage workflow (`.github/workflows/project-board-triage.yml`)
triggers on `mainnet` and `zk` label events. If labels are renamed or missing,
issues silently skip the automation.

**Action:** Verify #1255 and #1256 are complete and the `MAINNET_PROJECT_URL` /
`ZK_PROJECT_URL` repository variables are set before enabling the triage
workflow.

---

## Soft Dependencies (ordering recommended, not blocking)

| Recommended order | Reason |
|-------------------|--------|
| #1193 (ZK threat model) before #1197–#1210 | Rules are easier to scope when the threat model is agreed |
| #1112 (main audit) before mainnet freeze date | Audit findings may require changes that shift the freeze |
| #1182 (perf gate) before #1251 (ZK review) | ZK review findings may add rules; baseline should be set first |

---

## Dependency Graph (simplified)

```
[#1255 labels] ──┐
[#1256 milestones]─┤
                   └──► [#1260 board triage]

[#1192 Z-namespace ADR] ─────────────────────► [#1197–#1210 Z-rules]
                                                       │
[#1182 perf gate] ──────────────────────────────────► │ (both must land first)

[#1193 ZK threat model] ────────────────────► [#1197–#1210 Z-rules]

[#1251 ZK specialist review] ───────────────► [#1142 remove zk_features flag]

[#1112 main audit] ──► [mainnet freeze / deploy] (independent of #1142)
```

---

## Contacts

| Area | Point of contact |
|------|-----------------|
| Mainnet milestone (#1252) | Tag `@HyperSafeD` on the epic |
| ZK milestone (#1253) | ZK specialist assigned in #1251 |
| Performance gate (#1182) | CI / core-engine contributors |
| This document | Update via PR; link remains stable at `docs/WAVE_DEPENDENCIES.md` |
