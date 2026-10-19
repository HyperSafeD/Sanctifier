# Mainnet Readiness Plan

**Issues Addressed:** #1173, #1174, #1175, #1181  
**Status:** Implementation Guide  
**Target:** Mainnet Freeze

---

## Overview

This document provides a comprehensive plan for completing all mainnet readiness documentation, testing, and audit tasks before the mainnet freeze. The work is organized into four parallel tracks that can be executed independently.

---

## Track 1: Mainnet Quickstart Documentation (#1173)

**Priority:** P2-Medium  
**Difficulty:** Easy  
**Estimated Effort:** 1 day  
**Status:** ✅ COMPLETED IN THIS PR

### Implementation

#### 1.1 README.md Mainnet Quickstart Section

Added comprehensive mainnet quickstart section covering:
- Installation with mainnet-safe defaults
- Network configuration (`--confirm-mainnet` flag)
- Security checklist integration
- Migration guide references
- Safety flag documentation

#### 1.2 GETTING_STARTED.md Enhancements

Enhanced getting started guide with:
- Mainnet-specific workflow
- Production deployment checklist
- Safety verification steps
- Testnet-to-mainnet migration path

####  1.3 Translation Updates

Updated internationalized README files:
- `README.es.md` - Spanish translation
- `README.zh-CN.md` - Chinese translation
- Mainnet quickstart sections in all languages
- Consistent terminology across translations

### Acceptance Criteria

- [x] Mainnet quickstart section in README.md
- [x] Mainnet workflow in GETTING_STARTED.md
- [x] Spanish translation updated
- [x] Chinese translation updated
- [x] References to #1171 migration guide
- [x] References to #1124, #1133 safety flags

---

## Track 2: Documentation Link/Code Sample Audit (#1174)

**Priority:** P2-Medium  
**Difficulty:** Medium  
**Estimated Effort:** 2-3 days  
**Status:** 🔧 IMPLEMENTATION GUIDE PROVIDED

### Execution Plan

#### 2.1 Link Checking with Lychee

**Tool:** `lychee.toml` (already configured)

```bash
# Run full link check
lychee --config lychee.toml docs/ *.md

# Fix broken links incrementally
lychee --config lychee.toml --verbose docs/ 2>&1 | tee link-check-results.txt

# Generate report
lychee --config lychee.toml --format markdown docs/ > LINK_AUDIT_REPORT.md
```

**Common Link Issues:**
- GitHub issue/PR references: Update to current numbers
- External documentation: Check for moved/deprecated pages
- Code sample references: Verify file paths exist
- Anchor links: Validate section headers exist

#### 2.2 Code Sample Validation

**Tool:** `scripts/validate_docs_specs.js`

```bash
# Run validation script
node scripts/validate_docs_specs.js

# Fix issues found
# Common problems:
# - Outdated CLI command syntax
# - Removed API endpoints
# - Changed function signatures
# - Missing imports in code samples
```

#### 2.3 Manual Spot-Check (High-Traffic Docs)

**Files to Manually Test:**
1. `GETTING_STARTED.md` - All code samples
2. `QUICK_START.md` - Quick start commands
3. `docs/rules/S001.md` through `docs/rules/S012.md` - Rule examples
4. `README.md` - 30-second quickstart section
5. `LIVE_TESTNET.md` - On-chain invocation examples

**Test Against Mainnet-Stable CLI:**
```bash
# Verify each code sample executes without error
sanctifier analyze ./contracts
sanctifier analyze ./contracts --exit-code --format sarif
sanctifier badge --report report.json --svg-output sanctifier.svg

# Test on-chain commands (if applicable)
stellar contract invoke --network mainnet --id $CONTRACT_ID -- health_check
```

### Deliverables

- [ ] `LINK_AUDIT_REPORT.md` - Complete link check results
- [ ] `CODE_SAMPLE_VALIDATION_REPORT.md` - Validation script results
- [ ] All broken links fixed
- [ ] All code samples verified working
- [ ] `validate_docs_specs.js` passing clean
- [ ] `lychee` passing clean

### Acceptance Criteria

- [ ] Lychee link-check passes clean (zero broken links)
- [ ] `validate_docs_specs.js` passes clean (zero validation errors)
- [ ] High-traffic docs manually spot-checked and verified
- [ ] All fixes committed and tested

---

## Track 3: Video Walkthrough for Mainnet Workflow (#1175)

**Priority:** P3-Low  
**Difficulty:** Easy  
**Estimated Effort:** 1-2 days  
**Status:** 📹 SCRIPT PROVIDED

### Video Script Extension

**File:** `docs/VIDEO_WALKTHROUGH_SCRIPT.md`

#### Scene 1: Introduction (0:00 - 0:45)
```
"Welcome to Sanctifier's Mainnet Workflow demonstration.

In this walkthrough, we'll show you how to scan, deploy, and monitor
Soroban smart contracts on mainnet using Sanctifier's security tooling.

This extends our formal verification series with mainnet-specific
features introduced in the latest release."
```

#### Scene 2: Installation & Setup (0:45 - 2:00)
```
"First, install Sanctifier with mainnet-ready defaults:

[TERMINAL]
cargo install sanctifier-cli

Next, configure your network. Sanctifier requires an explicit
confirmation flag for mainnet operations:

[TERMINAL]
export STELLAR_NETWORK=mainnet
export SANCTIFIER_CONFIRM_MAINNET=true

Or use the --confirm-mainnet flag on each command."
```

#### Scene 3: Pre-Deployment Scan (2:00 - 4:00)
```
"Before deploying to mainnet, run a comprehensive security scan:

[TERMINAL]
sanctifier analyze ./contracts --network mainnet

Pay attention to critical findings marked S001-S012.
Each finding has documentation and remediation guidance.

[SHOW: Finding output with explanations]

Fix all critical issues before proceeding."
```

#### Scene 4: Mainnet Deployment (4:00 - 6:00)
```
"Deploy your contract with mainnet safety flags:

[TERMINAL]
stellar contract deploy \\
  --wasm target/wasm32-unknown-unknown/release/contract.wasm \\
  --network mainnet \\
  --source DEPLOYER_SECRET

Sanctifier monitors the deployment and validates:
- Contract bytecode integrity
- Initial state safety
- Authorization patterns
- Storage initialization

[SHOW: Deployment output]"
```

#### Scene 5: Post-Deployment Monitoring (6:00 - 8:00)
```
"After deployment, enable runtime monitoring:

[TERMINAL]
sanctifier monitor \\
  --contract-id CXXXXX... \\
  --network mainnet \\
  --alert-on critical,high

This provides:
- Real-time event monitoring
- Authorization audit trail
- State change validation
- Anomaly detection

[SHOW: Monitor dashboard]"
```

#### Scene 6: Verification & Next Steps (8:00 - 9:00)
```
"Verify your deployment with health checks:

[TERMINAL]
stellar contract invoke \\
  --id CXXXXX... \\
  --network mainnet \\
  -- health_check

For more information:
- Migration Guide: docs/MAINNET_MIGRATION_GUIDE.md
- Safety Checklist: docs/MAINNET_SAFETY_CHECKLIST.md
- Full documentation: sanctifier.dev

Thank you for using Sanctifier to secure Soroban mainnet."
```

### Production Checklist

- [ ] Script approved and reviewed
- [ ] Test environment setup (mainnet simulation)
- [ ] Recording software configured (OBS, Loom, etc.)
- [ ] Audio quality checked
- [ ] Screen resolution optimized (1920x1080 recommended)
- [ ] Terminal font size readable (16pt minimum)
- [ ] Video recorded and edited
- [ ] Captions/subtitles added
- [ ] Published to hosting channel (YouTube, Vimeo, etc.)
- [ ] Links updated in:
  - [ ] README.md
  - [ ] GETTING_STARTED.md
  - [ ] docs/MAINNET_MIGRATION_GUIDE.md (if exists)
  - [ ] docs/formal-verification-video-series.md

### Acceptance Criteria

- [ ] Video recorded, published, and linked from relevant docs
- [ ] Video covers complete mainnet workflow end-to-end
- [ ] Script follows existing series format
- [ ] All commands demonstrated are functional

---

## Track 4: Snapshot Test Coverage Audit (#1181)

**Priority:** P2-Medium  
**Difficulty:** Medium  
**Estimated Effort:** 3 days  
**Status:** 🔍 AUDIT FRAMEWORK PROVIDED

### Audit Methodology

#### 4.1 Rule Enumeration

**Script:** Create `scripts/audit_rule_coverage.sh`

```bash
#!/bin/bash
# Enumerate all active rules from tooling/sanctifier-core

echo "# Rule Coverage Audit Report"
echo "Generated: $(date)"
echo ""

# Extract all rule codes
RULES=$(grep -r "pub const.*: &str = \"S[0-9]" tooling/sanctifier-core/src/ | \
  sed 's/.*"\(S[0-9]*\)".*/\1/' | sort -u)

echo "## Active Rules"
echo ""
for rule in $RULES; do
  echo "- $rule"
done

echo ""
echo "Total rules: $(echo "$RULES" | wc -l)"
```

#### 4.2 Fixture Cross-Reference

**Location:** `contracts/fixtures/finding-codes/`

For each rule, verify:
1. **Triggering fixture** exists - Should produce finding
2. **Clean fixture** exists - Should NOT produce finding
3. **Snapshot test** passes for both

**Script:** Create `scripts/check_fixture_pairs.sh`

```bash
#!/bin/bash
# Check fixture pairs for each rule

FIXTURE_DIR="contracts/fixtures/finding-codes"
MISSING_PAIRS=()

for rule_dir in $FIXTURE_DIR/S???; do
  rule=$(basename $rule_dir)
  
  if [ ! -f "$rule_dir/triggering.rs" ]; then
    MISSING_PAIRS+=("$rule: missing triggering.rs")
  fi
  
  if [ ! -f "$rule_dir/clean.rs" ]; then
    MISSING_PAIRS+=("$rule: missing clean.rs")
  fi
  
  if [ ! -f "$rule_dir/test_snapshot.rs" ]; then
    MISSING_PAIRS+=("$rule: missing test_snapshot.rs")
  fi
done

if [ ${#MISSING_PAIRS[@]} -gt 0 ]; then
  echo "❌ Missing fixture pairs:"
  printf '%s\n' "${MISSING_PAIRS[@]}"
  exit 1
else
  echo "✅ All rules have complete fixture pairs"
fi
```

#### 4.3 Coverage Table Generation

**Output:** `docs/rules/COVERAGE_TABLE.md`

```markdown
# Rule Test Coverage Status

| Rule | Triggering Fixture | Clean Fixture | Snapshot Test | Status |
|------|-------------------|---------------|---------------|--------|
| S001 | ✅ | ✅ | ✅ | Complete |
| S002 | ✅ | ✅ | ✅ | Complete |
| S003 | ✅ | ✅ | ✅ | Complete |
| ... | ... | ... | ... | ... |
| S030 | ❌ | ❌ | ❌ | Missing |

**Legend:**
- ✅ Present and passing
- ⚠️ Present but failing
- ❌ Missing

**Summary:**
- Total rules: 30+
- Complete coverage: XX rules
- Partial coverage: XX rules
- Missing coverage: XX rules
```

### Missing Fixture Template

For rules without fixtures, use this template:

**File:** `contracts/fixtures/finding-codes/SXXX/triggering.rs`
```rust
// Triggering fixture for SXXX: [Rule Description]
// This code SHOULD produce a finding

#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct TriggeringContract;

#[contractimpl]
impl TriggeringContract {
    // Add code that violates SXXX rule
    pub fn violating_function(env: Env) {
        // TODO: Implement violation
    }
}
```

**File:** `contracts/fixtures/finding-codes/SXXX/clean.rs`
```rust
// Clean fixture for SXXX: [Rule Description]
// This code should NOT produce a finding

#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct CleanContract;

#[contractimpl]
impl CleanContract {
    // Add compliant code
    pub fn compliant_function(env: Env) {
        // TODO: Implement compliant version
    }
}
```

### Execution Plan

1. **Day 1: Audit**
   ```bash
   ./scripts/audit_rule_coverage.sh > RULE_AUDIT.md
   ./scripts/check_fixture_pairs.sh
   ```

2. **Day 2: Add Missing Fixtures**
   - Identify gaps from audit
   - Create triggering/clean pairs for each
   - Write snapshot tests

3. **Day 3: Verification & Documentation**
   ```bash
   cargo test --package sanctifier-core -- --nocapture
   ./scripts/generate_coverage_table.sh > docs/rules/COVERAGE_TABLE.md
   ```

### Deliverables

- [ ] `scripts/audit_rule_coverage.sh` - Rule enumeration script
- [ ] `scripts/check_fixture_pairs.sh` - Fixture verification script
- [ ] `docs/rules/COVERAGE_TABLE.md` - Coverage status table
- [ ] Missing fixture pairs added
- [ ] All snapshot tests passing
- [ ] Coverage report published

### Acceptance Criteria

- [ ] Every active rule S001-S030 has both triggering and clean fixtures
- [ ] All fixtures have passing snapshot tests
- [ ] Coverage table published in `docs/rules/`
- [ ] No gaps in test coverage

---

## Timeline & Dependencies

### Week 1
- **Day 1-2:** Track 1 (Mainnet Quickstart) - ✅ COMPLETE
- **Day 2-4:** Track 4 (Snapshot Audit) - Start audit phase
- **Day 5:** Track 2 (Link Audit) - Run lychee + validation script

### Week 2
- **Day 6-7:** Track 4 (Snapshot Audit) - Add missing fixtures
- **Day 7-8:** Track 2 (Link Audit) - Fix broken links/samples
- **Day 8-9:** Track 3 (Video) - Record and publish

### Dependencies

```
#1171 (Migration Guide)
  ↓
#1173 (Mainnet Quickstart) ← THIS PR
  ↓
#1175 (Video Walkthrough)

#1174 (Link Audit) ← Independent
#1181 (Test Coverage) ← Independent
```

---

## Success Metrics

- [ ] All documentation passes link checking (lychee clean)
- [ ] All code samples validated (validate_docs_specs.js clean)
- [ ] Mainnet quickstart available in 4 languages
- [ ] Video published and linked from docs
- [ ] 100% rule coverage with fixture pairs
- [ ] Coverage table published
- [ ] Zero gaps before mainnet freeze

---

## Notes for Future Contributors

### Quick Commands

```bash
# Full documentation audit
make lint-docs

# Link check only
lychee --config lychee.toml docs/ *.md

# Code sample validation
node scripts/validate_docs_specs.js

# Fixture coverage audit
cargo test --package sanctifier-core --lib -- --nocapture | grep "S[0-9]"

# Generate coverage report
./scripts/generate_coverage_table.sh
```

### Maintenance

- Run link checks monthly
- Update code samples with each CLI release
- Re-record video annually or after major features
- Audit fixture coverage before each release

---

**Last Updated:** $(date)  
**Maintainer:** HyperSafeD Team  
**Related Issues:** #1173, #1174, #1175, #1181
