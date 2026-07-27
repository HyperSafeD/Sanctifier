# CI Enhancements and ZK Rules Implementation Plan

**Status**: Implementation Framework
**Author**: boluwacodes
**Date**: 2026-07-27
**Issues Addressed**: #1180, #1182, #1198, #1202

---

## Executive Summary

This document provides comprehensive implementation frameworks for:
1. **Cross-network integration test matrix** (testnet, futurenet, mainnet-fork)
2. **Performance regression CI gate** (>10% analysis time regression blocker)
3. **ZK Rule Z002**: Insecure/predictable randomness in circuit/proof inputs
4. **ZK Rule Z006**: Missing proof nonce/uniqueness enforcement (replay attacks)

### Implementation Approach

- **Issues #1180, #1182**: Enhanced CI workflows with matrix testing and benchmark comparison
- **Issues #1198, #1202**: Implementation plans for ZK-specific detection rules (depend on #1192, #1194, #1197)

---

## Issue #1180: Cross-Network Integration Test Matrix

### Goal

Run the core integration suite against testnet, futurenet, and mainnet-fork to catch network-specific behavioral divergence before mainnet deployment.

### Current State Analysis

**Existing Infrastructure:**
- ✅ Testnet integration in `.github/workflows/soroban-examples.yml`
- ✅ Contract CI in `.github/workflows/contracts-ci.yml`
- ⚠️ Single network target (testnet primary)
- ⚠️ No futurenet or mainnet-fork coverage

### Implementation Plan

#### Phase 1: Create Parameterized Workflow (Day 1-2)

**File: `.github/workflows/cross-network-tests.yml`**

```yaml
name: Cross-Network Integration Tests

on:
  push:
    branches: [main, 'release/**']
  pull_request:
    branches: [main]
  workflow_dispatch:
    inputs:
      network:
        description: 'Network to test against'
        required: false
        default: 'all'
        type: choice
        options:
          - all
          - testnet
          - futurenet
          - mainnet-fork

env:
  CARGO_TERM_COLOR: always

jobs:
  test-matrix:
    name: Integration Tests - ${{ matrix.network }}
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        network:
          - testnet
          - futurenet
          - mainnet-fork
    
    steps:
      - uses: actions/checkout@v6
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      
      - name: Cache dependencies
        uses: actions/cache@v5
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ matrix.network }}-${{ hashFiles('**/Cargo.lock') }}

      
      - name: Install Soroban CLI
        run: cargo install --locked soroban-cli
      
      - name: Setup Network Configuration
        id: network-config
        run: |
          case "${{ matrix.network }}" in
            testnet)
              echo "rpc_url=https://soroban-testnet.stellar.org" >> $GITHUB_OUTPUT
              echo "network_passphrase=Test SDF Network ; September 2015" >> $GITHUB_OUTPUT
              echo "friendly_name=Testnet" >> $GITHUB_OUTPUT
              ;;
            futurenet)
              echo "rpc_url=https://rpc-futurenet.stellar.org" >> $GITHUB_OUTPUT
              echo "network_passphrase=Test SDF Future Network ; October 2022" >> $GITHUB_OUTPUT
              echo "friendly_name=Futurenet" >> $GITHUB_OUTPUT
              ;;
            mainnet-fork)
              echo "rpc_url=http://localhost:8000" >> $GITHUB_OUTPUT
              echo "network_passphrase=Public Global Stellar Network ; September 2015" >> $GITHUB_OUTPUT
              echo "friendly_name=Mainnet Fork" >> $GITHUB_OUTPUT
              ;;
          esac
      
      - name: Start Mainnet Fork (if applicable)
        if: matrix.network == 'mainnet-fork'
        run: |
          docker run -d \
            --name stellar-mainnet-fork \
            -p 8000:8000 \
            stellar/quickstart:latest \
            --standalone \
            --enable-soroban-rpc \
            --network-passphrase "Public Global Stellar Network ; September 2015"
          
          # Wait for RPC to be ready
          for i in {1..30}; do
            if curl -s http://localhost:8000/health | grep -q "ready"; then
              echo "Mainnet fork ready"
              break
            fi
            echo "Waiting for mainnet fork... ($i/30)"
            sleep 2
          done
      
      - name: Build contracts
        run: |
          cargo build --target wasm32-unknown-unknown --release \
            -p multisig-wallet -p amm-pool -p timelock -p vesting-contract \
            -p reentrancy-guard -p runtime-guard-wrapper
      
      - name: Run integration tests
        env:
          SOROBAN_RPC_URL: ${{ steps.network-config.outputs.rpc_url }}
          SOROBAN_NETWORK_PASSPHRASE: ${{ steps.network-config.outputs.network_passphrase }}
        run: |
          echo "Testing against ${{ steps.network-config.outputs.friendly_name }}"
          echo "RPC URL: $SOROBAN_RPC_URL"
          
          cargo test --package sanctifier-core --package sanctifier-cli \
            integration_tests \
            -- --nocapture --test-threads=1
      
      - name: Cleanup mainnet fork
        if: always() && matrix.network == 'mainnet-fork'
        run: |
          docker stop stellar-mainnet-fork || true
          docker rm stellar-mainnet-fork || true
      
      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v6
        with:
          name: test-results-${{ matrix.network }}
          path: |
            target/debug/deps/*.xml
            test-results-${{ matrix.network }}.json
          retention-days: 7
  
  summarize-results:
    name: Summarize Cross-Network Results
    needs: test-matrix
    runs-on: ubuntu-latest
    if: always()
    
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v6
        with:
          path: test-results/
      
      - name: Generate summary
        run: |
          echo "## Cross-Network Integration Test Summary" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          echo "| Network | Status |" >> $GITHUB_STEP_SUMMARY
          echo "|---------|--------|" >> $GITHUB_STEP_SUMMARY
          
          for network in testnet futurenet mainnet-fork; do
            if [ -d "test-results/test-results-$network" ]; then
              echo "| $network | ✅ PASS |" >> $GITHUB_STEP_SUMMARY
            else
              echo "| $network | ❌ FAIL |" >> $GITHUB_STEP_SUMMARY
            fi
          done
```


#### Phase 2: Network-Specific Test Fixtures (Day 2-3)

Create network-specific test configurations:

**File: `tests/fixtures/network-config.json`**

```json
{
  "testnet": {
    "rpc_url": "https://soroban-testnet.stellar.org",
    "network_passphrase": "Test SDF Network ; September 2015",
    "protocol_version": 20,
    "resource_limits": {
      "max_cpu_instructions": 100000000,
      "max_memory_bytes": 41943040
    }
  },
  "futurenet": {
    "rpc_url": "https://rpc-futurenet.stellar.org",
    "network_passphrase": "Test SDF Future Network ; October 2022",
    "protocol_version": 21,
    "resource_limits": {
      "max_cpu_instructions": 100000000,
      "max_memory_bytes": 41943040
    }
  },
  "mainnet": {
    "rpc_url": "https://mainnet.stellar.validationcloud.io/v1/<YOUR_API_KEY>",
    "network_passphrase": "Public Global Stellar Network ; September 2015",
    "protocol_version": 20,
    "resource_limits": {
      "max_cpu_instructions": 100000000,
      "max_memory_bytes": 41943040
    }
  }
}
```

#### Phase 3: Divergence Detection and Resolution (Day 4-7)

**Common Divergence Patterns:**

1. **Protocol Version Differences**
   - Futurenet may have newer protocol features
   - Solution: Feature-flag tests or skip with clear annotation

2. **Resource Limit Variations**
   - Different networks may have different CPU/memory limits
   - Solution: Parameterize tests with network-specific limits

3. **RPC Endpoint Availability**
   - Testnet/futurenet may have rate limits or downtime
   - Solution: Retry logic with exponential backoff

4. **State Consistency**
   - Mainnet fork starts from real state
   - Solution: Reset to clean state before each test

**Divergence Resolution Script:**

```bash
#!/bin/bash
# scripts/analyze-network-divergence.sh

set -euo pipefail

echo "Analyzing cross-network test results..."

RESULTS_DIR="test-results"
DIVERGENCES_FOUND=false

for network in testnet futurenet mainnet-fork; do
  RESULT_FILE="$RESULTS_DIR/test-results-$network.json"
  
  if [ ! -f "$RESULT_FILE" ]; then
    echo "⚠️  Missing results for $network"
    continue
  fi
  
  FAILURES=$(jq '.failures | length' "$RESULT_FILE")
  
  if [ "$FAILURES" -gt 0 ]; then
    echo "❌ $network: $FAILURES test(s) failed"
    DIVERGENCES_FOUND=true
    
    jq -r '.failures[] | "  - \(.test_name): \(.error)"' "$RESULT_FILE"
  else
    echo "✅ $network: All tests passed"
  fi
done

if [ "$DIVERGENCES_FOUND" = true ]; then
  echo ""
  echo "⚠️  Network divergences detected. Review failed tests and either:"
  echo "   1. Fix the divergence (preferred)"
  echo "   2. Document as expected behavior"
  echo "   3. Add network-specific test configuration"
  exit 1
fi

echo ""
echo "✅ No divergences found across all networks"
```

**Estimated Effort**: 5-7 days

---

## Issue #1182: Performance Regression CI Gate

### Goal

Fail CI builds if analysis time regresses more than 10% compared to the `main` baseline, preventing performance decay as new rules are added.

### Current State Analysis

**Existing Infrastructure:**
- ✅ Benchmarks exist in `tooling/sanctifier-core/benches`
- ✅ Workflow `.github/workflows/benchmarks.yml`
- ⚠️ Currently informational only, not blocking
- ⚠️ No baseline comparison mechanism

### Implementation Plan

#### Phase 1: Baseline Storage Strategy (Day 1)

**Option A: Git-based Baseline (Recommended)**

Store baseline results in a separate branch or file:

```bash
# scripts/store-benchmark-baseline.sh
#!/bin/bash
set -euo pipefail

BASELINE_FILE="benchmarks/baseline.json"
CURRENT_RESULTS="benchmarks/current.json"

# Run benchmarks
cd tooling/sanctifier-core
cargo bench --bench analysis_benchmarks -- --output-format json > "../../$CURRENT_RESULTS"

# Store as baseline (run this on main only)
if [ "${GITHUB_REF}" = "refs/heads/main" ]; then
  cp "$CURRENT_RESULTS" "$BASELINE_FILE"
  git add "$BASELINE_FILE"
  git commit -m "chore: Update benchmark baseline [skip ci]"
  git push
fi
```

**Option B: GitHub Actions Cache**

Store in workflow cache with expiry:

```yaml
- name: Restore baseline cache
  uses: actions/cache@v5
  with:
    path: benchmarks/baseline.json
    key: benchmark-baseline-${{ github.sha }}
    restore-keys: |
      benchmark-baseline-
```


#### Phase 2: Comparison Logic Implementation (Day 1-2)

**File: `scripts/compare-benchmarks.py`**

```python
#!/usr/bin/env python3
"""
Compare current benchmark results against baseline and fail if regression > 10%
"""
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple

REGRESSION_THRESHOLD = 0.10  # 10%

def load_benchmark_results(path: Path) -> Dict:
    """Load benchmark results from JSON file"""
    if not path.exists():
        print(f"Error: Benchmark file not found: {path}")
        sys.exit(1)
    
    with open(path, 'r') as f:
        return json.load(f)

def compare_benchmarks(baseline: Dict, current: Dict) -> List[Tuple[str, float, float, float]]:
    """
    Compare benchmark results and return regressions
    
    Returns: List of (benchmark_name, baseline_time, current_time, regression_pct)
    """
    regressions = []
    
    baseline_benches = {b['name']: b for b in baseline.get('benchmarks', [])}
    current_benches = {b['name']: b for b in current.get('benchmarks', [])}
    
    for name, baseline_bench in baseline_benches.items():
        if name not in current_benches:
            print(f"⚠️  Warning: Benchmark '{name}' missing in current results")
            continue
        
        current_bench = current_benches[name]
        
        baseline_time = baseline_bench.get('mean', {}).get('estimate', 0)
        current_time = current_bench.get('mean', {}).get('estimate', 0)
        
        if baseline_time == 0:
            continue
        
        regression = (current_time - baseline_time) / baseline_time
        
        if regression > REGRESSION_THRESHOLD:
            regressions.append((name, baseline_time, current_time, regression))
    
    return regressions

def format_time(nanoseconds: float) -> str:
    """Format time in human-readable units"""
    if nanoseconds < 1000:
        return f"{nanoseconds:.2f} ns"
    elif nanoseconds < 1_000_000:
        return f"{nanoseconds / 1000:.2f} µs"
    elif nanoseconds < 1_000_000_000:
        return f"{nanoseconds / 1_000_000:.2f} ms"
    else:
        return f"{nanoseconds / 1_000_000_000:.2f} s"

def main():
    baseline_path = Path("benchmarks/baseline.json")
    current_path = Path("benchmarks/current.json")
    
    if not baseline_path.exists():
        print("⚠️  No baseline found. This is the first run.")
        print("Storing current results as baseline...")
        if current_path.exists():
            current_path.rename(baseline_path)
        sys.exit(0)
    
    baseline = load_benchmark_results(baseline_path)
    current = load_benchmark_results(current_path)
    
    regressions = compare_benchmarks(baseline, current)
    
    if not regressions:
        print("✅ No performance regressions detected")
        print(f"All benchmarks within {REGRESSION_THRESHOLD * 100}% of baseline")
        sys.exit(0)
    
    print(f"❌ Performance regression detected!")
    print(f"The following benchmarks regressed more than {REGRESSION_THRESHOLD * 100}%:\n")
    
    for name, baseline_time, current_time, regression in regressions:
        print(f"Benchmark: {name}")
        print(f"  Baseline: {format_time(baseline_time)}")
        print(f"  Current:  {format_time(current_time)}")
        print(f"  Regression: {regression * 100:.2f}%")
        print()
    
    print("To override this check (if regression is justified):")
    print("  1. Review and approve the performance impact")
    print("  2. Add '[skip perf check]' to your commit message")
    print("  3. Or update the baseline: bash scripts/update-baseline.sh")
    
    sys.exit(1)

if __name__ == "__main__":
    main()
```


#### Phase 3: Enhanced Benchmark Workflow (Day 2)

Update `.github/workflows/benchmarks.yml`:

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch: {}

env:
  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true

jobs:
  benchmark:
    name: Run Benchmarks
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0  # Need history for baseline
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install System Dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libz3-dev pkg-config
      
      - name: Rust Cache
        uses: Swatinem/rust-cache@v2
      
      - name: Fetch baseline from main branch
        if: github.ref != 'refs/heads/main'
        run: |
          git fetch origin main:main
          git show main:benchmarks/baseline.json > benchmarks/baseline.json || echo "{}" > benchmarks/baseline.json
      
      - name: Run benchmarks
        run: |
          cd tooling/sanctifier-core
          cargo bench --bench analysis_benchmarks -- --output-format json \
            | tee ../../benchmarks/current.json
      
      - name: Compare against baseline
        if: github.event_name == 'pull_request'
        run: |
          python3 scripts/compare-benchmarks.py
      
      - name: Check for skip flag
        if: failure() && contains(github.event.head_commit.message, '[skip perf check]')
        run: |
          echo "⚠️  Performance check skipped by commit message flag"
          exit 0
      
      - name: Update baseline (main branch only)
        if: github.ref == 'refs/heads/main'
        run: |
          cp benchmarks/current.json benchmarks/baseline.json
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add benchmarks/baseline.json
          git commit -m "chore: Update benchmark baseline [skip ci]" || echo "No changes"
          git push || echo "Push failed, may need permissions"
      
      - name: Upload benchmark results
        if: always()
        uses: actions/upload-artifact@v6
        with:
          name: benchmark-results
          path: benchmarks/*.json
          retention-days: 30
      
      - name: Comment on PR
        if: failure() && github.event_name == 'pull_request'
        uses: actions/github-script@v8
        with:
          script: |
            const fs = require('fs');
            
            let comment = '## ⚠️ Performance Regression Detected\n\n';
            comment += 'One or more benchmarks have regressed by more than 10%.\n\n';
            comment += 'See the workflow logs for detailed comparison.\n\n';
            comment += '**To override**: Add `[skip perf check]` to your commit message if this regression is justified.\n';
            
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: comment
            });
```

**Estimated Effort**: 2-3 days

---

## Issue #1198: ZK Rule Z002 - Insecure Randomness in Circuit Inputs

### Goal

Detect usage of predictable on-chain randomness sources (ledger timestamp, sequence number) as secret/blinding input to commitment or nullifier construction.

### Dependencies

- **#1192**: ZK taint-tracking infrastructure foundation
- **#1194**: Commitment/nullifier sink function identification
- **Related**: S018 (unsafe-prng) rule for reusable patterns

### Current State Analysis

This rule requires:
1. Taint source identification (ledger timestamp, sequence number)
2. Taint propagation through data flow
3. Sink detection (commitment/nullifier constructors)
4. Pattern matching for ZK-specific contexts

### Implementation Plan

#### Phase 1: Taint Source Identification (Day 1)

**Predictable Randomness Sources to Track:**

```rust
// Taint sources for Z002
enum PredictableSource {
    LedgerTimestamp,       // env.ledger().timestamp()
    SequenceNumber,        // env.ledger().sequence()
    BlockNumber,           // env.ledger().block_number()  
    TransactionHash,       // env.current_contract_address() (semi-predictable)
    AccountSequence,       // account.sequence()
}
```

**Detection Pattern:**

```rust
// contracts/fixtures/finding-codes/z002_insecure_randomness.rs

// FLAGGED: Ledger timestamp as commitment secret
pub fn insecure_commitment_timestamp(env: Env, amount: u64) -> BytesN<32> {
    let secret = env.ledger().timestamp();  // ❌ Predictable source
    let commitment = poseidon_hash(&env, &[secret.into(), amount.into()]);
    commitment
}

// FLAGGED: Sequence number as nullifier input
pub fn insecure_nullifier_sequence(env: Env, note_id: u64) -> BytesN<32> {
    let salt = env.ledger().sequence();  // ❌ Predictable source
    let nullifier = keccak256(&env, &[note_id.into(), salt.into()]);
    nullifier
}

// CLEAN: User-supplied secret
pub fn secure_commitment_user_secret(env: Env, secret: BytesN<32>, amount: u64) -> BytesN<32> {
    // ✅ Secret from user, not predictable
    let commitment = poseidon_hash(&env, &[secret, amount.into()]);
    commitment
}

// CLEAN: Cryptographically secure random
pub fn secure_commitment_csprng(env: Env, amount: u64) -> BytesN<32> {
    let secret = env.prng().gen_range(0..u64::MAX);  // ✅ Proper PRNG
    let commitment = poseidon_hash(&env, &[secret.into(), amount.into()]);
    commitment
}
```


#### Phase 2: Sink Function Identification (Day 2)

**ZK Commitment/Nullifier Sink Functions:**

```rust
// Sink functions that should NOT receive predictable inputs
const ZK_SINK_FUNCTIONS: &[&str] = &[
    // Commitment constructors
    "poseidon_hash",
    "pedersen_commit",
    "commitment_create",
    "note_commitment",
    
    // Nullifier constructors
    "nullifier_derive",
    "spend_nullifier",
    "note_nullifier",
    
    // Generic hash used in ZK context
    "keccak256",  // Only when used for commitment/nullifier
    "sha256",     // Only when used for commitment/nullifier
    "blake3",     // Only when used for commitment/nullifier
];
```

**Context-Aware Detection:**

The rule must distinguish between:
- ❌ Hash used for commitment/nullifier (ZK context)
- ✅ Hash used for general data integrity (non-ZK context)

This requires control-flow and data-flow analysis to identify ZK-specific usage patterns.

#### Phase 3: Taint Propagation Logic (Day 3-4)

**Taint Tracking Algorithm:**

```python
# Pseudocode for Z002 taint tracking

def detect_insecure_zk_randomness(contract_ast):
    """
    Detect predictable randomness used in ZK commitment/nullifier construction
    """
    findings = []
    
    # Step 1: Identify taint sources
    taint_sources = find_predictable_sources(contract_ast)
    # Returns: [(var_name, source_type, location)]
    
    # Step 2: Propagate taint through data flow
    tainted_vars = propagate_taint(contract_ast, taint_sources)
    # Returns: {var_name: (source_type, propagation_path)}
    
    # Step 3: Check if tainted data reaches ZK sinks
    for sink_call in find_zk_sink_calls(contract_ast):
        for arg in sink_call.arguments:
            if arg.var_name in tainted_vars:
                source_type, path = tainted_vars[arg.var_name]
                
                # Verify this is actually a ZK context
                if is_zk_context(sink_call):
                    findings.append({
                        'rule': 'Z002',
                        'severity': 'HIGH',
                        'location': sink_call.location,
                        'message': f'Predictable {source_type} used as ZK secret input',
                        'taint_path': path,
                        'sink_function': sink_call.function_name
                    })
    
    return findings

def find_predictable_sources(ast):
    """Find calls to predictable randomness sources"""
    sources = []
    
    for call in ast.find_all_calls():
        if matches_pattern(call, 'env.ledger().timestamp()'):
            sources.append((call.result_var, 'timestamp', call.location))
        elif matches_pattern(call, 'env.ledger().sequence()'):
            sources.append((call.result_var, 'sequence', call.location))
        # ... other patterns
    
    return sources

def is_zk_context(sink_call):
    """
    Determine if a hash/commitment call is in a ZK context
    
    Heuristics:
    - Variable names contain 'commitment', 'nullifier', 'note'
    - Return value used in 'verify_proof' or 'check_nullifier'
    - Function comment mentions ZK/privacy
    """
    # Check variable naming
    if any(keyword in sink_call.result_var.lower() 
           for keyword in ['commitment', 'nullifier', 'note', 'secret']):
        return True
    
    # Check usage in downstream calls
    uses = find_variable_uses(sink_call.result_var)
    for use in uses:
        if any(keyword in use.function_name 
               for keyword in ['verify', 'nullifier', 'proof']):
            return True
    
    return False
```


#### Phase 4: Rule Documentation (Day 4)

**File: `docs/rules/Z002.md`**

```markdown
# Z002: Insecure Randomness in ZK Circuit Inputs

## Severity
**HIGH** - Can completely break privacy guarantees

## Description
Detects usage of predictable on-chain data (ledger timestamp, sequence numbers) as secret input to ZK commitment or nullifier construction. An attacker can predict or brute-force these values, breaking the privacy or uniqueness guarantee.

## Vulnerable Pattern

```rust
// ❌ BAD: Ledger timestamp as commitment secret
let secret = env.ledger().timestamp();
let commitment = poseidon_hash(&env, &[secret.into(), amount.into()]);
```

The attacker can observe the ledger timestamp and reconstruct the commitment, revealing the hidden amount.

## Secure Pattern

```rust
// ✅ GOOD: User-supplied secret
pub fn create_note(env: Env, secret: BytesN<32>, amount: u64) {
    let commitment = poseidon_hash(&env, &[secret, amount.into()]);
    // User provides cryptographically random secret
}

// ✅ GOOD: Proper CSPRNG
let secret = env.prng().gen_range(0..u64::MAX);
let commitment = poseidon_hash(&env, &[secret.into(), amount.into()]);
```

## Why This Matters

ZK systems rely on secrets being unpredictable. Predictable randomness:
- **Breaks privacy**: Attacker can reconstruct commitments
- **Enables double-spending**: Attacker can predict nullifiers
- **Defeats purpose**: The ZK scheme provides no security

## Detection Method

1. **Taint sources**: Track variables derived from `env.ledger().timestamp()`, `.sequence()`, etc.
2. **Taint propagation**: Follow data flow through assignments and function calls
3. **Sink detection**: Flag when tainted data reaches commitment/nullifier constructors
4. **Context verification**: Ensure detection is ZK-specific (not general hashing)

## Related Rules
- **S018** (unsafe-prng): General PRNG security (reuses taint infrastructure)
- **Z001**: Missing nullifier checks
- **Z003**: Public input leakage

## References
- [ZK Privacy Best Practices](https://zkp.science)
- [Predictable Randomness in Smart Contracts](https://arxiv.org/abs/2002.12043)
```

**Estimated Effort**: 3-4 days (after #1192, #1194 dependencies)

---

## Issue #1202: ZK Rule Z006 - Missing Proof Nonce/Uniqueness

### Goal

Detect proof-gated privileged actions (non-transfer) lacking nonce or context-binding, allowing the same proof to be replayed across different transactions or contexts.

### Dependencies

- **#1192**: ZK infrastructure foundation
- **#1194**: Proof verification pattern detection
- **#1197**: Z001 implementation (to differentiate from value-transfer replay)

### Distinction from Z001

- **Z001**: Double-spend prevention via nullifiers (value transfer specific)
- **Z006**: Replay prevention via nonces (general privileged actions)

**Example Scenarios:**
- Governance voting (vote same proof multiple times)
- Identity attestation (reuse proof across contexts)
- Access control (replay authorization proof)
- Delegation (resubmit delegation proof)


### Implementation Plan

#### Phase 1: Privileged Action Pattern Detection (Day 1-2)

**Identify Non-Transfer Proof-Gated Actions:**

```rust
// contracts/fixtures/finding-codes/z006_replay_missing_nonce.rs

// FLAGGED: Governance vote without nonce
pub fn vote_on_proposal(
    env: Env,
    proposal_id: u64,
    proof: BytesN<32>,
    public_inputs: Vec<u64>
) {
    // Verify ZK proof of voting eligibility
    verify_zk_proof(&env, proof, public_inputs);
    
    // ❌ No nonce check - same proof can be replayed!
    let votes = env.storage().get(&proposal_id).unwrap_or(0);
    env.storage().set(&proposal_id, votes + 1);
}

// FLAGGED: Identity attestation without context binding
pub fn attest_identity(
    env: Env,
    user: Address,
    proof: BytesN<32>,
    public_inputs: Vec<u64>
) {
    verify_identity_proof(&env, proof, public_inputs);
    
    // ❌ No context binding - proof valid everywhere!
    env.storage().set(&user, true);
}

// CLEAN: Vote with nonce enforcement
pub fn vote_on_proposal_secure(
    env: Env,
    proposal_id: u64,
    proof: BytesN<32>,
    public_inputs: Vec<u64>,
    nonce: u64
) {
    verify_zk_proof(&env, proof, public_inputs);
    
    // ✅ Check nonce has not been used
    let used_nonces = get_used_nonces(&env);
    if used_nonces.contains(&nonce) {
        panic!("Nonce already used");
    }
    
    // Mark nonce as used
    used_nonces.insert(nonce);
    env.storage().set(&USED_NONCES_KEY, used_nonces);
    
    // Process vote
    let votes = env.storage().get(&proposal_id).unwrap_or(0);
    env.storage().set(&proposal_id, votes + 1);
}

// CLEAN: Attestation with context binding
pub fn attest_identity_secure(
    env: Env,
    user: Address,
    proof: BytesN<32>,
    public_inputs: Vec<u64>,
    context: BytesN<32>  // Binds proof to specific contract/purpose
) {
    // Verify proof includes context commitment
    verify_identity_proof_with_context(&env, proof, public_inputs, context);
    
    // ✅ Proof is context-bound, can't be reused elsewhere
    env.storage().set(&user, true);
}
```


#### Phase 2: Nonce/Context-Binding Detection Logic (Day 2-3)

**Detection Algorithm:**

```python
# Pseudocode for Z006 detection

def detect_missing_proof_nonce(contract_ast):
    """
    Detect proof-gated privileged actions without nonce/context-binding
    """
    findings = []
    
    # Step 1: Find all proof verification sites
    proof_verifications = find_proof_verifications(contract_ast)
    
    for verification in proof_verifications:
        # Step 2: Determine if this is a privileged action
        if not is_privileged_action(verification):
            continue
        
        # Step 3: Check if this is a value-transfer (skip, covered by Z001)
        if is_value_transfer(verification):
            continue
        
        # Step 4: Check for nonce enforcement
        has_nonce = check_nonce_enforcement(verification)
        
        # Step 5: Check for context binding
        has_context_binding = check_context_binding(verification)
        
        if not has_nonce and not has_context_binding:
            findings.append({
                'rule': 'Z006',
                'severity': 'HIGH',
                'location': verification.location,
                'message': 'Proof-gated privileged action without nonce or context binding',
                'action_type': verification.action_type,
                'suggestion': 'Add nonce tracking or bind proof to specific context'
            })
    
    return findings

def is_privileged_action(verification):
    """
    Determine if this is a privileged action (not just read-only)
    
    Heuristics:
    - Modifies storage
    - Changes contract state
    - Affects other users
    - Grants permissions
    """
    function = verification.containing_function
    
    # Check for storage writes
    if function.has_storage_writes():
        return True
    
    # Check for permission grants
    if any(keyword in function.name.lower() 
           for keyword in ['grant', 'approve', 'authorize', 'vote', 'delegate']):
        return True
    
    # Check for state-changing keywords
    if function.modifies_state():
        return True
    
    return False

def check_nonce_enforcement(verification):
    """
    Check if nonce is tracked and enforced
    
    Must verify:
    1. Nonce parameter exists
    2. Nonce is checked against used-nonce storage
    3. Nonce is marked as used after verification
    """
    function = verification.containing_function
    
    # Check for nonce parameter
    has_nonce_param = any('nonce' in param.name.lower() 
                          for param in function.parameters)
    
    if not has_nonce_param:
        return False
    
    # Check for nonce validation logic
    has_nonce_check = function.contains_pattern(
        'storage.get.*nonce.*contains'
    ) or function.contains_pattern(
        'used_nonces.*contains'
    )
    
    if not has_nonce_check:
        return False
    
    # Check nonce is marked as used
    has_nonce_storage = function.contains_pattern(
        'storage.set.*nonce'
    ) or function.contains_pattern(
        'used_nonces.*insert'
    )
    
    return has_nonce_storage

def check_context_binding(verification):
    """
    Check if proof is bound to specific context
    
    Context binding methods:
    - Contract address included in public inputs
    - Purpose/action hash included in proof
    - Transaction-specific data in proof
    """
    function = verification.containing_function
    
    # Check for context parameter
    has_context_param = any(
        keyword in param.name.lower() 
        for param in function.parameters
        for keyword in ['context', 'binding', 'domain']
    )
    
    if not has_context_param:
        return False
    
    # Check context is verified in proof
    has_context_verification = function.contains_pattern(
        'verify.*context'
    ) or function.contains_pattern(
        'public_inputs.*context'
    )
    
    return has_context_verification

def is_value_transfer(verification):
    """
    Determine if this is a value transfer (covered by Z001)
    
    Heuristics:
    - Function name contains 'transfer', 'withdraw', 'send'
    - Involves token amounts
    - Nullifier pattern present
    """
    function = verification.containing_function
    
    transfer_keywords = ['transfer', 'withdraw', 'send', 'spend', 'redeem']
    
    if any(keyword in function.name.lower() for keyword in transfer_keywords):
        return True
    
    # Check for nullifier pattern (Z001 territory)
    if function.contains_pattern('nullifier'):
        return True
    
    return False
```


#### Phase 3: Rule Documentation (Day 3-4)

**File: `docs/rules/Z006.md`**

```markdown
# Z006: Missing Proof Nonce/Uniqueness Enforcement

## Severity
**HIGH** - Enables replay attacks on privileged actions

## Description
Detects proof-gated privileged actions (governance votes, identity attestation, access control) that lack nonce or context-binding checks, allowing the same proof to be replayed across different transactions or contexts.

## Vulnerable Pattern

```rust
// ❌ BAD: Vote without nonce - can be replayed
pub fn vote(env: Env, proposal_id: u64, proof: BytesN<32>, inputs: Vec<u64>) {
    verify_zk_proof(&env, proof, inputs);
    
    // No nonce check - attacker can resubmit same proof!
    let votes = env.storage().get(&proposal_id).unwrap_or(0);
    env.storage().set(&proposal_id, votes + 1);
}
```

The attacker can:
1. Obtain a valid proof once
2. Replay it multiple times to vote repeatedly
3. Manipulate governance outcomes

## Secure Patterns

### Option 1: Nonce Tracking

```rust
// ✅ GOOD: Nonce enforcement
pub fn vote(
    env: Env, 
    proposal_id: u64, 
    proof: BytesN<32>, 
    inputs: Vec<u64>,
    nonce: u64
) {
    verify_zk_proof(&env, proof, inputs);
    
    // Check nonce hasn't been used
    let used_nonces: Set<u64> = env.storage()
        .get(&USED_NONCES_KEY)
        .unwrap_or_default();
    
    if used_nonces.contains(&nonce) {
        panic!("Nonce already used");
    }
    
    // Mark nonce as used
    used_nonces.insert(nonce);
    env.storage().set(&USED_NONCES_KEY, used_nonces);
    
    // Process vote
    let votes = env.storage().get(&proposal_id).unwrap_or(0);
    env.storage().set(&proposal_id, votes + 1);
}
```

### Option 2: Context Binding

```rust
// ✅ GOOD: Context binding
pub fn vote(
    env: Env, 
    proposal_id: u64, 
    proof: BytesN<32>, 
    inputs: Vec<u64>,
    context: BytesN<32>  // Hash of (contract_address, proposal_id, action)
) {
    // Verify proof includes context commitment
    verify_proof_with_context(&env, proof, inputs, context);
    
    // Proof is bound to this specific context, can't be replayed elsewhere
    let votes = env.storage().get(&proposal_id).unwrap_or(0);
    env.storage().set(&proposal_id, votes + 1);
}
```

## Why This Matters

Without replay protection:
- **Governance manipulation**: Vote multiple times with one proof
- **Identity fraud**: Reuse attestation across platforms
- **Access abuse**: Replay authorization indefinitely
- **Delegation attacks**: Resubmit delegation proof after revocation

## Distinction from Z001

- **Z001**: Value transfer replay (requires nullifiers for double-spend prevention)
- **Z006**: Non-transfer replay (requires nonces for general uniqueness)

Both are replay attacks but in different contexts.

## Detection Method

1. **Find proof verifications**: Identify all `verify_proof` calls
2. **Check if privileged**: Determine if action modifies state or grants permissions
3. **Exclude transfers**: Skip value-transfer patterns (covered by Z001)
4. **Check nonce**: Look for nonce parameter and used-nonce tracking
5. **Check context**: Look for context-binding in proof verification
6. **Flag if missing**: Report if neither nonce nor context binding present

## Related Rules
- **Z001**: Missing nullifier checks (value-transfer specific)
- **Z003**: Public input leakage
- **S012**: Missing access control

## References
- [ZK Replay Attack Prevention](https://zkp.science/replay)
- [Context Binding in ZK Proofs](https://eprint.iacr.org/2023/456)
```

**Estimated Effort**: 3-4 days (after #1192, #1194, #1197 dependencies)

---

## Implementation Timeline

### Overall Schedule

```
Week 1:
  Day 1-2: #1180 Cross-network test matrix setup
  Day 3-4: #1180 Network-specific fixtures and divergence detection
  Day 5: #1182 Benchmark baseline storage strategy

Week 2:
  Day 1-2: #1182 Comparison logic and workflow enhancement
  Day 3: #1182 Testing and validation
  Day 4-5: #1198 Z002 taint source and sink identification

Week 3:
  Day 1-2: #1198 Z002 taint propagation logic
  Day 3: #1198 Z002 documentation and testing
  Day 4-5: #1202 Z006 privileged action detection

Week 4:
  Day 1-2: #1202 Z006 nonce/context-binding logic
  Day 3: #1202 Z006 documentation and testing
  Day 4-5: Integration testing and final validation
```

**Total Estimated Effort**: 3-4 weeks

**Dependencies:**
- #1198 and #1202 require #1192, #1194 to be completed first
- #1202 requires #1197 (Z001) to avoid duplicate findings
- #1180 and #1182 can proceed independently

---

## Success Criteria

### Issue #1180: Cross-Network Testing
- [ ] CI matrix runs against testnet, futurenet, mainnet-fork
- [ ] Results reported per-network
- [ ] Any divergences resolved or documented
- [ ] Workflow passes on sample contracts

### Issue #1182: Performance Regression Gate
- [ ] Baseline stored and retrievable
- [ ] Comparison logic detects >10% regressions
- [ ] CI fails on synthetic regression test
- [ ] Override mechanism documented and working

### Issue #1198: Z002 Rule
- [ ] Detects predictable randomness in ZK commitments
- [ ] Taint tracking from source to sink functional
- [ ] Clean patterns not flagged
- [ ] Snapshot tests pass
- [ ] Documentation complete

### Issue #1202: Z006 Rule
- [ ] Detects missing nonce in proof-gated actions
- [ ] Distinguishes from Z001 (no duplicate findings)
- [ ] Context-binding patterns recognized
- [ ] Snapshot tests pass
- [ ] Documentation complete

---

## Risk Mitigation

### Cross-Network Testing Challenges
- **Risk**: Network downtime or rate limiting
- **Mitigation**: Retry logic with exponential backoff
- **Fallback**: Document network-specific skips with reasons

### Benchmark Baseline Drift
- **Risk**: Baseline becomes stale or inaccurate
- **Mitigation**: Auto-update on main branch merges
- **Fallback**: Manual baseline update script

### ZK Rule Complexity
- **Risk**: High false positive/negative rate
- **Mitigation**: Extensive fixture testing, manual review of patterns
- **Fallback**: Mark as experimental until refined

### Dependency Delays
- **Risk**: #1192, #1194, #1197 not ready
- **Mitigation**: Implement stub infrastructure for testing
- **Fallback**: Deliver framework, defer rule activation

---

**Document Version**: 1.0
**Last Updated**: 2026-07-27
**Next Review**: Upon completion of Phase 1 for each issue
