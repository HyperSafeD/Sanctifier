# Testing Gates Implementation Plan

**Status**: Implementation Framework
**Author**: miraclesonly
**Date**: 2026-07-27
**Issues Addressed**: #1176, #1177, #1178, #1179

---

## Executive Summary

This document provides a comprehensive implementation framework for establishing four critical testing gates required before mainnet release of the Sanctifier protocol. These gates ensure the detection engine meets production-grade quality, safety, and reliability standards.

### Issues Covered

1. **#1176**: Enforce 90%+ coverage threshold on `sanctifier-core` as mainnet-freeze gate
2. **#1177**: Add full regression E2E suite: contract upload → scan → mainnet deploy
3. **#1178**: Add mutation-testing score as mainnet release blocker in CI
4. **#1179**: Run extended (24h+) fuzz campaign on core parser before freeze

### Implementation Approach

Given the nature of these issues (requiring extended execution time, manual campaigns, and infrastructure setup), this PR provides:

- **Configuration frameworks** for CI gates
- **Step-by-step implementation guides** with scripts
- **Test templates and scaffolding** for E2E scenarios
- **Campaign execution checklists** for fuzzing and mutation testing
- **Timeline estimates** and resource requirements

---

## Issue #1176: Enforce 90%+ Coverage Threshold on Core Engine

### Current State Analysis

**Existing Coverage Infrastructure:**
- ✅ Workflow: `.github/workflows/e2e-coverage.yml`
- ✅ Tool: `cargo-tarpaulin` for coverage generation
- ✅ Integration: Codecov upload configured
- ⚠️ Gap: No package-specific threshold enforcement

### Implementation Plan

#### Phase 1: Measure Current Coverage (Day 1)

**Script: `scripts/measure-core-coverage.sh`**

```bash
#!/bin/bash
set -euo pipefail

echo "📊 Measuring current coverage for sanctifier-core..."

cd tooling/sanctifier-core

# Generate detailed coverage report
cargo tarpaulin \
  --package sanctifier-core \
  --out Html \
  --out Json \
  --output-dir ../../coverage-reports/core \
  --engine llvm \
  --exclude-files "*/tests/*" "*/benches/*"

# Parse JSON for current coverage percentage
COVERAGE=$(jq '.files | map(.covered_percent) | add / length' coverage-reports/core/tarpaulin-report.json)

echo "Current sanctifier-core coverage: ${COVERAGE}%"

if (( $(echo "$COVERAGE < 90" | bc -l) )); then
  echo "⚠️  Coverage below 90% threshold"
  echo "Gap to close: $(echo "90 - $COVERAGE" | bc)%"
else
  echo "✅ Coverage meets 90% threshold"
fi
```


#### Phase 2: Configure codecov.yml Threshold (Day 1-2)

Create or update `codecov.yml` in repository root:

```yaml
coverage:
  status:
    project:
      default:
        target: 80%  # Overall project threshold
        threshold: 2%
    
    # Package-specific thresholds
    patch:
      default:
        target: 70%
  
  # Critical: sanctifier-core must maintain 90%+
  flags:
    sanctifier-core:
      paths:
        - tooling/sanctifier-core/src/
      target: 90%
      threshold: 0%  # No degradation allowed
      if_ci_failed: error

  precision: 2
  round: down
  range: "70...100"

comment:
  layout: "reach,diff,flags"
  behavior: default
  require_changes: false
```


#### Phase 3: Update CI Workflow (Day 2)

Modify `.github/workflows/e2e-coverage.yml` to enforce threshold:

```yaml
      - name: Generate Coverage Report with Threshold Check
        run: |
          cargo tarpaulin \
            --package sanctifier-core \
            --out Xml \
            --out Html \
            --fail-under 90 \
            --exclude-files "*/tests/*" "*/benches/*"
        
      - name: Verify Core Coverage Threshold
        run: |
          COVERAGE=$(cargo tarpaulin --package sanctifier-core --print-summary | grep -oP '\d+\.\d+(?=%)')
          if (( $(echo "$COVERAGE < 90" | bc -l) )); then
            echo "❌ FAIL: sanctifier-core coverage ($COVERAGE%) below 90% threshold"
            exit 1
          fi
          echo "✅ PASS: sanctifier-core coverage: $COVERAGE%"
```


#### Phase 4: Close Coverage Gaps (Day 2-5)

**Gap Analysis Workflow:**

1. Run coverage with detailed file breakdown:
   ```bash
   cargo tarpaulin --package sanctifier-core --out Html --output-dir coverage-html
   open coverage-html/index.html
   ```

2. Identify uncovered code paths:
   - Error handling branches
   - Edge cases in parser logic
   - Panic/abort paths
   - Complex conditional logic

3. Add targeted unit tests for each gap:
   ```rust
   // Example: Test error path coverage
   #[test]
   fn test_parse_invalid_utf8_source() {
       let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
       let result = parse_source(&invalid_bytes);
       assert!(matches!(result, Err(ParseError::InvalidUtf8)));
   }
   ```

4. Iteratively test and measure until 90%+ achieved

**Estimated Effort**: 3-5 days depending on gap size


#### Phase 5: Reference in Release Checklist

Update `docs/RELEASE_CHECKLIST.md` (create if doesn't exist):

```markdown
## Mainnet Freeze Prerequisites

### Testing Gates (MUST PASS)

- [ ] **Coverage Gate**: `sanctifier-core` coverage ≥ 90% (enforced in CI)
  - Workflow: `.github/workflows/e2e-coverage.yml`
  - Config: `codecov.yml` flag `sanctifier-core`
  - Latest report: Check Codecov dashboard
  
- [ ] **Mutation Testing Gate**: Kill rate ≥ 70% (see #1178)
- [ ] **Extended Fuzz Campaign**: 24h+ completed with zero crashes (see #1179)
- [ ] **E2E Regression Suite**: Full deploy flow passing (see #1177)
```

---

## Issue #1177: Add Full Regression E2E Suite

### Current State Analysis

**Existing E2E Infrastructure:**
- ✅ Framework: Playwright (implied from `e2e/tests/call-graph.spec.ts`)
- ✅ Workflow: `.github/workflows/e2e-coverage.yml`
- ⚠️ Gap: No mainnet deploy flow coverage


### Implementation Plan

#### Phase 1: Set Up Mainnet Fork/Sandbox (Day 1-2)

**Option A: Soroban Standalone (Recommended)**

```bash
#!/bin/bash
# scripts/setup-test-network.sh

docker run -d \
  --name soroban-standalone \
  -p 8000:8000 \
  stellar/quickstart:soroban-dev@sha256:latest \
  --standalone \
  --enable-soroban-rpc

# Wait for network to be ready
until curl -s http://localhost:8000/health | grep -q "ready"; do
  echo "Waiting for Soroban RPC..."
  sleep 2
done

echo "✅ Test network ready at http://localhost:8000"
```

**Option B: Testnet with Reset**

Use dedicated testnet account with periodic balance reset for E2E tests.


#### Phase 2: Create E2E Test Scaffold (Day 2-3)

**File: `e2e/tests/full-deploy-flow.spec.ts`**

```typescript
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import path from 'path';

test.describe('Full Deploy Flow: Upload → Scan → Mainnet Deploy', () => {
  let testContractPath: string;
  let scanResultPath: string;
  
  test.beforeAll(async () => {
    // Setup test network
    execSync('bash scripts/setup-test-network.sh');
    
    // Prepare test contract
    testContractPath = path.join(__dirname, '../fixtures/test-contract.wasm');
  });
  
  test('complete flow from upload to guarded mainnet deploy', async ({ page }) => {
    // Step 1: Upload contract to dashboard
    await page.goto('http://localhost:3000/dashboard');
    await page.click('[data-testid="upload-contract-btn"]');
    
    const fileInput = await page.locator('input[type="file"]');
    await fileInput.setInputFiles(testContractPath);
    
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible();
    
    // Step 2: Initiate scan
    await page.click('[data-testid="scan-btn"]');
    await expect(page.locator('[data-testid="scan-in-progress"]')).toBeVisible();
    
    // Wait for scan completion (with timeout)
    await expect(page.locator('[data-testid="scan-complete"]')).toBeVisible({ timeout: 60000 });
    
    // Step 3: Verify findings displayed
    const findingsCount = await page.locator('[data-testid="finding-item"]').count();
    console.log(`Scan found ${findingsCount} findings`);
    
    // Step 4: Download SARIF report
    const downloadPromise = page.waitForEvent('download');
    await page.click('[data-testid="download-sarif-btn"]');
    const download = await downloadPromise;
    scanResultPath = await download.path();
    expect(scanResultPath).toBeTruthy();
    
    // Step 5: Attempt mainnet deploy with safety gates
    await page.click('[data-testid="deploy-mainnet-btn"]');
    
    // Verify safety gate: --confirm-mainnet flag required
    await expect(page.locator('[data-testid="mainnet-confirmation-modal"]')).toBeVisible();
    await expect(page.locator('text=/confirm.*mainnet/i')).toBeVisible();
    
    // Enter confirmation passphrase
    await page.fill('[data-testid="confirmation-input"]', 'DEPLOY_TO_MAINNET');
    await page.click('[data-testid="confirm-deploy-btn"]');
    
    // Wait for deployment transaction
    await expect(page.locator('[data-testid="deploy-success"]')).toBeVisible({ timeout: 30000 });
    
    // Verify contract address displayed
    const contractAddress = await page.locator('[data-testid="contract-address"]').textContent();
    expect(contractAddress).toMatch(/^C[A-Z0-9]{55}$/);
    
    console.log(`✅ Contract deployed to: ${contractAddress}`);
  });
  
  test.afterAll(async () => {
    // Cleanup test network
    execSync('docker stop soroban-standalone && docker rm soroban-standalone');
  });
});
```


#### Phase 3: Create Test Fixtures (Day 3-4)

**Directory: `e2e/fixtures/`**

1. **Test Contract**: Simple Soroban contract with known vulnerabilities
   ```bash
   # Compile from contracts/test-samples/ or use pre-built WASM
   cd contracts/test-samples/reentrancy-example
   soroban contract build
   cp target/wasm32-unknown-unknown/release/*.wasm ../../e2e/fixtures/test-contract.wasm
   ```

2. **Expected SARIF Output**: `e2e/fixtures/expected-sarif.json`
   ```json
   {
     "version": "2.1.0",
     "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
     "runs": [{
       "tool": {
         "driver": {
           "name": "Sanctifier"
         }
       },
       "results": [
         {
           "ruleId": "REENTRANCY_RISK",
           "level": "warning",
           "message": {
             "text": "Potential reentrancy vulnerability detected"
           }
         }
       ]
     }]
   }
   ```


#### Phase 4: Wire into CI (Day 4-5)

Update `.github/workflows/e2e-coverage.yml`:

```yaml
  e2e-full-flow:
    name: E2E Full Deploy Flow
    runs-on: ubuntu-latest
    if: github.ref == 'refs/tags/v*' || github.ref == 'refs/heads/release/*'
    
    steps:
      - uses: actions/checkout@v6
      
      - name: Setup Node.js
        uses: actions/setup-node@v6
        with:
          node-version: '20'
      
      - name: Install dependencies
        run: npm ci
        working-directory: e2e
      
      - name: Install Playwright browsers
        run: npx playwright install --with-deps
        working-directory: e2e
      
      - name: Setup test network
        run: bash scripts/setup-test-network.sh
      
      - name: Run full deploy flow test
        run: npx playwright test full-deploy-flow.spec.ts
        working-directory: e2e
        env:
          TEST_NETWORK_URL: http://localhost:8000
      
      - name: Upload test artifacts
        if: always()
        uses: actions/upload-artifact@v6
        with:
          name: e2e-test-results
          path: e2e/test-results/
          retention-days: 30
```

**Estimated Effort**: 5-7 days total


---

## Issue #1178: Add Mutation-Testing Score as Release Blocker

### Current State Analysis

**Existing Mutation Testing:**
- ✅ Workflow: `.github/workflows/mutation-testing.yml`
- ✅ Tool: `cargo-mutants`
- ✅ Runs: Weekly schedule + manual dispatch
- ⚠️ Status: `continue-on-error: true` (non-blocking)
- ⚠️ Gap: No threshold enforcement

### Implementation Plan

#### Phase 1: Define Target Threshold (Day 1)

Based on `cargo-mutants` best practices:
- **Minimum Kill Rate**: 70% for production code
- **Target Kill Rate**: 80%+ for critical paths
- **sanctifier-core specific**: 75% minimum

**Rationale:**
- Industry standard for mutation testing: 60-80%
- Higher than line coverage due to quality focus
- Balanced against CI time constraints


#### Phase 2: Update Workflow for Release Candidates (Day 1-2)

Modify `.github/workflows/mutation-testing.yml`:

```yaml
name: Mutation Testing

on:
  schedule:
    - cron: '0 0 * * 1'  # Weekly
  workflow_dispatch:
  push:
    tags:
      - 'v*'  # Run on version tags
    branches:
      - 'release/**'  # Run on release branches

jobs:
  mutation-test:
    name: Mutation Testing (cargo-mutants)
    runs-on: ubuntu-latest
    # Remove continue-on-error for release candidates
    continue-on-error: ${{ github.ref_type != 'tag' && !startsWith(github.ref, 'refs/heads/release/') }}

    steps:
      - name: Checkout repository
        uses: actions/checkout@v6
        with:
          fetch-depth: 0

      - name: Install stable Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Install Z3
        run: sudo apt-get update && sudo apt-get install -y libz3-dev

      - name: Cache cargo registry & build artifacts
        uses: actions/cache@v5
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-mutants-${{ hashFiles('**/Cargo.lock') }}

      - name: Install cargo-mutants
        run: cargo install cargo-mutants || true

      - name: Run mutation testing on sanctifier-core
        run: |
          cd tooling/sanctifier-core
          cargo mutants --no-shuffle --timeout 600 --output mutants.out
        env:
          CARGO_TERM_COLOR: always

      - name: Parse and validate mutation score
        id: mutation-score
        run: |
          cd tooling/sanctifier-core
          
          # Parse mutants.out for statistics
          CAUGHT=$(grep -c "caught" mutants.out/*.txt || echo "0")
          TOTAL=$(grep -c "^" mutants.out/*.txt || echo "1")
          
          KILL_RATE=$(echo "scale=2; ($CAUGHT / $TOTAL) * 100" | bc)
          
          echo "caught=$CAUGHT" >> $GITHUB_OUTPUT
          echo "total=$TOTAL" >> $GITHUB_OUTPUT
          echo "kill_rate=$KILL_RATE" >> $GITHUB_OUTPUT
          
          echo "📊 Mutation Testing Results:"
          echo "   Mutants Killed: $CAUGHT / $TOTAL"
          echo "   Kill Rate: $KILL_RATE%"

      - name: Enforce threshold for release builds
        if: github.ref_type == 'tag' || startsWith(github.ref, 'refs/heads/release/')
        run: |
          KILL_RATE=${{ steps.mutation-score.outputs.kill_rate }}
          THRESHOLD=75
          
          if (( $(echo "$KILL_RATE < $THRESHOLD" | bc -l) )); then
            echo "❌ FAIL: Mutation kill rate ($KILL_RATE%) below threshold ($THRESHOLD%)"
            echo "Cannot release with insufficient mutation testing coverage"
            exit 1
          fi
          
          echo "✅ PASS: Mutation kill rate ($KILL_RATE%) meets threshold ($THRESHOLD%)"
```


#### Phase 3: Close Surviving Mutant Gaps (Day 2-4)

**Gap Identification Workflow:**

1. Run mutation testing locally:
   ```bash
   cd tooling/sanctifier-core
   cargo mutants --no-shuffle
   ```

2. Analyze surviving mutants:
   ```bash
   # Review mutants.out/mutants.txt for survivors
   grep "survived" mutants.out/mutants.txt
   ```

3. Common surviving mutant patterns:
   - **Boundary conditions**: Off-by-one errors not caught
   - **Return value changes**: Functions that return unused values
   - **Logic inversions**: Missing negative test cases
   - **Timeout mutants**: Tests too slow to catch mutation

4. Add targeted tests for each survivor:
   ```rust
   #[test]
   fn test_boundary_condition_upper_limit() {
       // Test exact upper boundary
       assert!(validate_size(MAX_SIZE));
       assert!(!validate_size(MAX_SIZE + 1));
   }
   ```

5. Iterate until 75%+ kill rate achieved

**Estimated Effort**: 2-4 days depending on gap size


#### Phase 4: Document in MUTATION-TESTING.md

Create `docs/MUTATION-TESTING.md`:

```markdown
# Mutation Testing Guide

## Overview

Mutation testing validates that tests actually catch bugs by introducing controlled
mutations (bugs) into the code and verifying tests fail.

## Threshold Requirements

### sanctifier-core
- **Minimum Kill Rate**: 75%
- **Target Kill Rate**: 80%+
- **Enforced**: On release tags and release branches
- **Tool**: `cargo-mutants`

## Running Locally

```bash
cd tooling/sanctifier-core
cargo mutants --no-shuffle --timeout 600
```

## Interpreting Results

- **Caught**: Test suite detected the mutation ✅
- **Survived**: Mutation not detected (test gap) ⚠️
- **Timeout**: Test took too long (potential perf issue) ⏱️
- **Unviable**: Mutation caused compile error (expected) ℹ️

## Improving Kill Rate

1. Add tests for boundary conditions
2. Test error paths and edge cases
3. Verify return values are actually used
4. Add negative test cases

```

**Total Estimated Effort**: 3-4 days


---

## Issue #1179: Run Extended (24h+) Fuzz Campaign

### Current State Analysis

**Existing Fuzz Infrastructure:**
- ✅ Directory: `tooling/sanctifier-core/fuzz/`
- ✅ Tool: `cargo-fuzz` (libFuzzer)
- ✅ Workflows: `.github/workflows/fuzz.yml`, `.github/workflows/contracts-fuzz.yml`
- ⚠️ Duration: Short CI runs (5-10 minutes)
- ⚠️ Gap: No extended campaign documentation

### Implementation Plan

#### Phase 1: Pre-Campaign Setup (Day 1)

**Script: `scripts/run-extended-fuzz.sh`**

```bash
#!/bin/bash
set -euo pipefail

DURATION_HOURS=${1:-24}
FUZZ_TARGET=${2:-"all"}

echo "🔍 Starting extended fuzz campaign"
echo "Duration: ${DURATION_HOURS} hours"
echo "Target: ${FUZZ_TARGET}"

cd tooling/sanctifier-core/fuzz

# Create campaign output directory
CAMPAIGN_DIR="campaigns/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$CAMPAIGN_DIR"

# Log system information
uname -a > "$CAMPAIGN_DIR/system-info.txt"
rustc --version >> "$CAMPAIGN_DIR/system-info.txt"
cargo fuzz --version >> "$CAMPAIGN_DIR/system-info.txt"

# Build fuzz targets
cargo fuzz build

# List available targets
TARGETS=$(cargo fuzz list)
echo "Available fuzz targets:"
echo "$TARGETS"

if [ "$FUZZ_TARGET" = "all" ]; then
  FUZZ_TARGETS=$TARGETS
else
  FUZZ_TARGETS=$FUZZ_TARGET
fi

# Calculate seconds for duration
DURATION_SECONDS=$((DURATION_HOURS * 3600))

echo "Starting campaign at $(date)"
echo "$DURATION_SECONDS" > "$CAMPAIGN_DIR/planned-duration.txt"
date +%s > "$CAMPAIGN_DIR/start-time.txt"

# Run each target in parallel (if multiple cores available)
for target in $FUZZ_TARGETS; do
  echo "Fuzzing target: $target"
  
  cargo fuzz run "$target" \
    --jobs $(nproc) \
    --release \
    -- \
    -max_total_time="$DURATION_SECONDS" \
    -print_final_stats=1 \
    -artifact_prefix="$CAMPAIGN_DIR/" \
    2>&1 | tee "$CAMPAIGN_DIR/${target}.log" &
  
  # Store PID for monitoring
  echo $! >> "$CAMPAIGN_DIR/pids.txt"
done

echo "Campaign running in background. Monitor with:"
echo "  tail -f $CAMPAIGN_DIR/*.log"
echo ""
echo "To stop campaign:"
echo "  kill \$(cat $CAMPAIGN_DIR/pids.txt)"
```


#### Phase 2: Run Campaign (Day 2-3, 24h+ wall-clock)

**Execution Checklist:**

- [ ] **Choose execution environment**:
  - Option A: Dedicated development machine (8+ cores recommended)
  - Option B: Cloud runner (AWS EC2 c5.2xlarge or similar)
  - Option C: GitHub Actions with self-hosted runner (longer retention)

- [ ] **Launch campaign**:
  ```bash
  # From repo root
  bash scripts/run-extended-fuzz.sh 24 all
  ```

- [ ] **Monitor progress**:
  ```bash
  # Check current stats
  tail -f tooling/sanctifier-core/fuzz/campaigns/*/fuzz_target_*.log
  
  # Check for crashes
  ls -lah tooling/sanctifier-core/fuzz/campaigns/*/crash-*
  ls -lah tooling/sanctifier-core/fuzz/campaigns/*/leak-*
  ls -lah tooling/sanctifier-core/fuzz/campaigns/*/timeout-*
  ```

- [ ] **Keep machine awake**:
  ```bash
  # On macOS
  caffeinate -i bash scripts/run-extended-fuzz.sh 24 all
  
  # On Linux (with systemd-inhibit)
  systemd-inhibit bash scripts/run-extended-fuzz.sh 24 all
  ```


#### Phase 3: Triage Findings (Day 4-6)

**Crash Analysis Workflow:**

1. **Catalog all crashes/hangs/leaks**:
   ```bash
   cd tooling/sanctifier-core/fuzz/campaigns/[latest]
   
   # Count findings
   echo "Crashes: $(ls crash-* 2>/dev/null | wc -l)"
   echo "Timeouts: $(ls timeout-* 2>/dev/null | wc -l)"
   echo "Leaks: $(ls leak-* 2>/dev/null | wc -l)"
   ```

2. **Reproduce each finding**:
   ```bash
   # Replay specific crash
   cargo fuzz run [target] crash-abc123def456
   ```

3. **Categorize findings**:
   - **Critical**: Panic/abort in production code
   - **High**: Memory leak or infinite loop
   - **Medium**: Unhandled error case
   - **Low**: Test harness artifact (not production bug)

4. **Fix and verify**:
   ```rust
   // Example fix for panic on malformed input
   pub fn parse_source(input: &[u8]) -> Result<Ast, ParseError> {
       // Before: input.len() > MAX_SIZE (panic on overflow)
       // After: Safe checked arithmetic
       let len = input.len();
       if len > MAX_SIZE {
           return Err(ParseError::InputTooLarge);
       }
       // ... rest of parsing
   }
   ```

5. **Add regression fixture**:
   ```bash
   # Copy crash input to corpus for permanent regression testing
   cp crash-abc123def456 ../corpus/[target]/regression-001
   ```


#### Phase 4: Document Campaign Results (Day 6-7)

Create `docs/FUZZ_CAMPAIGN_[DATE].md`:

```markdown
# Extended Fuzz Campaign Report

**Campaign ID**: [YYYYMMDD-HHMMSS]
**Duration**: 24 hours 15 minutes
**Date**: 2026-07-[XX] to 2026-07-[YY]
**Executor**: [Name/Team]

## Configuration

- **Targets**: `parse_contract_source`, `analyze_control_flow`, `detect_vulnerabilities`
- **Jobs**: 8 parallel
- **Environment**: AWS EC2 c5.2xlarge (8 vCPU, 16GB RAM)
- **Corpus Size (Initial)**: 1,234 inputs
- **Corpus Size (Final)**: 2,567 inputs (+1,333 new interesting inputs)

## Results Summary

| Metric | Count |
|--------|-------|
| Total Executions | 45,823,991 |
| Exec/sec (avg) | 523 |
| Crashes | 3 |
| Timeouts | 1 |
| Leaks | 0 |
| New Coverage | 12 new edges |

## Findings

### Critical: Parse Buffer Overflow (FIXED)
- **File**: `src/parser/lexer.rs:234`
- **Issue**: Unchecked slice indexing on malformed UTF-8
- **Fix**: PR #[XXXX]
- **Corpus**: Added to `corpus/parse_source/regression-001`

### High: Infinite Loop on Cyclic AST (FIXED)
- **File**: `src/analyzer/graph.rs:89`
- **Issue**: Cycle detection missed certain patterns
- **Fix**: PR #[XXXX]
- **Corpus**: Added to `corpus/analyze_control_flow/regression-002`

### Medium: Unhandled Large Contract (FIXED)
- **File**: `src/detector/engine.rs:156`
- **Issue**: Stack overflow on deeply nested structures
- **Fix**: Iterative algorithm, PR #[XXXX]
- **Corpus**: Added to `corpus/detect_vulns/regression-003`

## Corpus Commitment

All new interesting inputs committed to repository:
```bash
git add tooling/sanctifier-core/fuzz/corpus/
git commit -m "fuzz: Add corpus from 24h campaign [DATE]"
```

## Conclusion

✅ Campaign completed successfully with zero unresolved findings.
All crashes, timeouts, and edge cases addressed and fixed.
New corpus committed for continuous regression protection.
```

**Total Estimated Effort**: 1 week calendar time (mostly automated wall-clock)


---

## Integration Timeline

### Overall Implementation Schedule

```
Week 1:
  Day 1-2: #1176 Coverage threshold setup + measurement
  Day 3-5: #1176 Close coverage gaps to 90%
  Day 6-7: #1178 Mutation testing threshold config

Week 2:
  Day 1-3: #1178 Close surviving mutant gaps
  Day 4-5: #1177 E2E test scaffold and fixtures
  Day 6-7: #1177 E2E CI integration

Week 3:
  Day 1: #1179 Fuzz campaign setup
  Day 2-3: #1179 Run 24h+ campaign (automated)
  Day 4-6: #1179 Triage and fix findings
  Day 7: #1179 Document and commit corpus

Week 4:
  Day 1-2: Integration testing of all gates
  Day 3-4: Update RELEASE_CHECKLIST.md
  Day 5: Final validation and documentation
```

**Total Estimated Effort**: 3-4 weeks calendar time


---

## Resource Requirements

### Human Resources
- **Senior Engineer**: 2-3 weeks (implementation lead)
- **QA Engineer**: 1 week (test design and execution)
- **DevOps Engineer**: 3-5 days (CI configuration and cloud setup)

### Infrastructure
- **Cloud Resources**: AWS EC2 instance for 24h+ ($50-100)
- **CI/CD Minutes**: ~500 additional minutes/month (GitHub Actions)
- **Storage**: ~5GB for corpus and campaign artifacts

### Tools
- ✅ cargo-tarpaulin (installed)
- ✅ cargo-mutants (installed)
- ✅ cargo-fuzz (installed)
- ⚠️ Playwright (needs setup for E2E)
- ⚠️ Soroban standalone (needs Docker setup)

---

## Success Criteria

### Gate #1: Coverage Threshold (#1176)
- [x] `codecov.yml` configured with 90% threshold for `sanctifier-core`
- [ ] CI workflow enforces threshold on every PR
- [ ] Current coverage meets or exceeds 90%
- [ ] Documented in `RELEASE_CHECKLIST.md`

### Gate #2: E2E Regression Suite (#1177)
- [ ] E2E test covers upload → scan → deploy flow
- [ ] Test runs against Soroban standalone/testnet
- [ ] Safety gates (`--confirm-mainnet`, passphrase) exercised
- [ ] CI runs on release-candidate builds
- [ ] Test passes with ✅ status

### Gate #3: Mutation Testing Score (#1178)
- [ ] Workflow updated to block on release tags/branches
- [ ] Threshold set to 75% minimum kill rate
- [ ] Current kill rate meets or exceeds threshold
- [ ] Documented in `docs/MUTATION-TESTING.md`

### Gate #4: Extended Fuzz Campaign (#1179)
- [ ] Campaign script created and tested
- [ ] 24h+ campaign completed successfully
- [ ] All crashes/panics/hangs fixed
- [ ] New corpus committed to repository
- [ ] Campaign report documented in `docs/FUZZ_CAMPAIGN_[DATE].md`


---

## Risk Mitigation

### Coverage Gap Larger Than Expected (#1176)
- **Risk**: sanctifier-core coverage currently below 80%
- **Mitigation**: Prioritize high-impact code paths first
- **Fallback**: Adjust threshold to 85% for initial release, plan 90% for v1.1

### E2E Test Environment Issues (#1177)
- **Risk**: Soroban standalone unstable or incompatible
- **Mitigation**: Use dedicated testnet account as backup
- **Fallback**: Document manual testing checklist if automated E2E blocks release

### Low Initial Mutation Kill Rate (#1178)
- **Risk**: Kill rate currently 40-50% (far from 75%)
- **Mitigation**: Focus on fixing highest-value survivors first
- **Fallback**: Set initial threshold at 60%, roadmap to 75% in phases

### Fuzz Campaign Finds Critical Bugs (#1179)
- **Risk**: 24h campaign discovers unfixable architectural issues
- **Mitigation**: Run shorter (4h) pre-campaign to surface shallow issues early
- **Fallback**: Document findings, assess severity, decide on release delay vs. mitigation

---

## Dependencies

### Issue Dependencies
- #1177 depends on #1133, #1135, #1136 (guarded mainnet deploy implementation)
- #1176, #1178, #1179 feed into #1140 (mainnet freeze gate)
- All issues feed into #1186 (mainnet release checklist)

### External Dependencies
- Soroban RPC endpoint availability (for E2E tests)
- GitHub Actions runner capacity (for extended CI runs)
- Rust toolchain stability (cargo-fuzz, cargo-mutants compatibility)

---

## Review and Approval Checklist

Before merging this implementation plan:

- [ ] Technical feasibility reviewed by Rust team
- [ ] Resource allocation approved by project management
- [ ] Timeline alignment with mainnet release schedule
- [ ] CI capacity verified with DevOps
- [ ] Security team consulted on fuzz target selection
- [ ] Documentation standards reviewed

---

## Appendix: Quick Start Commands

### Run All Checks Locally

```bash
# Coverage check (sanctifier-core)
cd tooling/sanctifier-core
cargo tarpaulin --out Html --output-dir coverage-html

# Mutation testing (short run)
cd tooling/sanctifier-core
cargo mutants --no-shuffle --timeout 60

# Fuzz testing (1-hour trial)
cd tooling/sanctifier-core/fuzz
cargo fuzz run [target] -- -max_total_time=3600

# E2E test (after setup)
cd e2e
npx playwright test full-deploy-flow.spec.ts
```

### Monitor CI Status

```bash
# Check latest coverage report
open https://app.codecov.io/gh/HyperSafeD/Sanctifier

# Check mutation testing artifacts
gh run list --workflow=mutation-testing.yml --limit 5

# Check E2E test results
gh run list --workflow=e2e-coverage.yml --limit 5
```

---

**Document Version**: 1.0
**Last Updated**: 2026-07-27
**Next Review**: Upon completion of Phase 1 of any gate
