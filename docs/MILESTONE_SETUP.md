# Milestone Setup: "Mainnet Launch Readiness" & "ZK Integration"

> **Issue #1256** — Verification record for the two Next Wave GitHub milestones.

This document confirms that both milestones are correctly configured and tracks
the verification pass performed at wave kickoff.

---

## Milestones

### Mainnet Launch Readiness

| Field | Value |
|-------|-------|
| **Title** | Mainnet Launch Readiness |
| **Epic** | #1252 |
| **Description** | Tracks all work required before Sanctifier is ready for a mainnet deployment: security audit (#1112), performance regression gate (#1182), deployment hardening, and operator runbook finalisation. |
| **Due date** | Not set (per team preference; adjust when freeze date is confirmed) |
| **Issue range** | #1112–#1181, #1183–#1191, #1244–#1255 (mainnet-scoped) |

### ZK Integration

| Field | Value |
|-------|-------|
| **Title** | ZK Integration |
| **Epic** | #1253 |
| **Description** | Tracks the full ZK workstream: Z-rule namespace ADR (#1192), ZK threat model (#1193), Z-rules Z001–Z099 (#1197–#1210 and follow-ups), rule-registry wiring (#1230), ZK specialist review (#1251), and feature-flag removal (#1142). |
| **Due date** | Not set (gated on #1251 specialist availability) |
| **Issue range** | #1142, #1192–#1243 (ZK-scoped) |

---

## Verification Checklist

- [x] Both milestones exist in the GitHub repository
- [x] Milestone descriptions are accurate and link to their epics
- [x] `Mainnet Launch Readiness` milestone assigned to all mainnet-scoped issues
- [x] `ZK Integration` milestone assigned to all ZK-scoped issues
- [x] Open/closed issue counts visible on the milestone page reflect actual progress
- [x] `Next Wave — Program Ops` milestone used for ops/tooling issues (#1256–#1260) that support both epics without belonging to either workstream

---

## How to Assign Issues in Bulk

If issues need to be reassigned after milestone creation, use the GitHub CLI:

```bash
# Assign a single issue to a milestone (get milestone number from GitHub UI)
gh api repos/HyperSafeD/Sanctifier/issues/<issue_number> \
  --method PATCH \
  --field milestone=<milestone_number>

# Bulk-assign a range (example: issues 1192–1210 to ZK Integration milestone 3)
for i in $(seq 1192 1210); do
  gh api repos/HyperSafeD/Sanctifier/issues/$i \
    --method PATCH \
    --field milestone=3
done
```

---

## Cross-Reference

See [`docs/WAVE_DEPENDENCIES.md`](WAVE_DEPENDENCIES.md) for the cross-milestone
dependency graph that explains which workstreams block each other.
