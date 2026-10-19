# ZK Rules Phase 2: Implementation Plan

**Status**: Implementation Framework  
**Author**: prodbycorne  
**Date**: 2026-07-27  
**Issues Addressed**: #1197 (Z001), #1203 (Z007), #1205 (Z009), #1207 (Z011)

---

## Executive Summary

This document provides comprehensive implementation frameworks for four critical ZK detection rules. All depend on #1192 (ZK infrastructure) and #1194 (proof-verification detection).

**Priority Order**: Z001 (CRITICAL) → Z009 (HIGH) → Z011 (MEDIUM) → Z007 (EXPERT, requires #1227)

---

## Issue #1197: Z001 - Missing Nullifier Check (CRITICAL)

### Goal
Detect proof verification followed by state mutation without nullifier checks, enabling double-spend attacks.

### Vulnerable Pattern
```rust
// ❌ No nullifier check before transfer
pub fn claim(env: Env, proof: Proof, public_inputs: Vec<Fr>) {
    require_valid_proof(&env, &proof, &public_inputs);
    transfer_funds(&env, &public_inputs); // Can replay!
}
```

### Secure Pattern
```rust
// ✅ Nullifier check prevents replay
pub fn claim(env: Env, proof: Proof, nullifier: BytesN<32>, public_inputs: Vec<Fr>) {
    require_valid_proof(&env, &proof, &public_inputs);
    
    // Check nullifier not used
    let used_nullifiers: Set<BytesN<32>> = env.storage().get(&NULLIFIERS_KEY).unwrap_or_default();
    if used_nullifiers.contains(&nullifier) {
        panic!("Nullifier already used");
    }
    
    // Record nullifier
    used_nullifiers.insert(nullifier);
    env.storage().set(&NULLIFIERS_KEY, &used_nullifiers);
    
    // Now safe to transfer
    transfer_funds(&env, &public_inputs);
}
```

### Detection Strategy
1. Find all proof verification calls
2. Identify subsequent state mutations (transfer, mint, claim)
3. Check for nullifier storage operations between verification and mutation
4. Flag if nullifier check/insert missing

### Implementation Effort
**4-5 days** (after #1192, #1194)

---

## Issue #1203: Z007 - Under-Constrained Circuits (EXPERT)

### Goal
Detect circuit signals used in arithmetic/comparisons without range-check constraints, enabling field-overflow attacks.

### Vulnerable Pattern (Circom)
```circom
// ❌ No range constraint on amount
signal input amount;
signal output isValid;
isValid <== amount < balance; // Field overflow possible!
```

### Secure Pattern (Circom)
```circom
// ✅ Range check enforced
signal input amount;
signal output isValid;

// Enforce amount is in valid range (e.g., 64 bits)
component rangeCheck = Num2Bits(64);
rangeCheck.in <== amount;

// Now safe to compare
isValid <== amount < balance;
```

### Detection Strategy
1. Parse circom/Noir circuit source (requires #1227 circom parser)
2. Build signal dataflow graph
3. Identify signals used in arithmetic/comparison operations
4. Check for preceding range-check templates (Num2Bits, LessThan)
5. Flag signals lacking constraints

### Dependencies
- **#1227**: Circom parser integration (blocks implementation)
- **#1192, #1194**: ZK infrastructure

### Implementation Effort
**1-2 weeks** (after #1227 parser available)

---

## Issue #1205: Z009 - Unbounded Proof-Verification Loops

### Goal
Detect loops that verify caller-controlled unbounded number of proofs, causing CPU budget exhaustion.

### Vulnerable Pattern
```rust
// ❌ Unbounded batch verification
pub fn batch_claim(env: Env, proofs: Vec<Proof>) {
    for proof in proofs.iter() {
        verify_proof(&env, &proof); // No length cap!
    }
    // Can hit CPU limit, DoS contract
}
```

### Secure Pattern
```rust
// ✅ Bounded with maximum batch size
const MAX_BATCH_SIZE: u32 = 10;

pub fn batch_claim(env: Env, proofs: Vec<Proof>) {
    if proofs.len() > MAX_BATCH_SIZE {
        panic!("Batch size exceeds maximum");
    }
    
    for proof in proofs.iter() {
        verify_proof(&env, &proof);
    }
}
```

### Detection Strategy
1. Find loops iterating over collections
2. Check if loop body calls proof-verification functions
3. Determine if collection length is:
   - Bounded by compile-time constant
   - Checked at runtime with maximum
   - Unbounded (from caller input)
4. Flag unbounded verification loops

### Implementation Effort
**2-3 days** (after #1192, #1194)

---

## Issue #1207: Z011 - Missing Domain Separation

### Goal
Detect multiple commitment constructions using same hash function without domain-separation tags, enabling cross-context collision attacks.

### Vulnerable Pattern
```rust
// ❌ Same hash for different purposes
fn create_note_commitment(amount: u64, secret: u64) -> BytesN<32> {
    poseidon_hash(&[amount.into(), secret.into()])
}

fn create_nullifier(note_id: u64, secret: u64) -> BytesN<32> {
    poseidon_hash(&[note_id.into(), secret.into()]) // Same pattern!
}

// Attacker can find collision between commitment and nullifier
```

### Secure Pattern
```rust
// ✅ Domain separation tags distinguish contexts
const DOMAIN_COMMITMENT: u64 = 0;
const DOMAIN_NULLIFIER: u64 = 1;

fn create_note_commitment(amount: u64, secret: u64) -> BytesN<32> {
    poseidon_hash(&[DOMAIN_COMMITMENT.into(), amount.into(), secret.into()])
}

fn create_nullifier(note_id: u64, secret: u64) -> BytesN<32> {
    poseidon_hash(&[DOMAIN_NULLIFIER.into(), note_id.into(), secret.into()])
}
```

### Detection Strategy
1. Find all commitment/nullifier construction sites in project
2. Group by hash function used (poseidon, pedersen, etc.)
3. Analyze input patterns for each group
4. Check if first argument is a domain-separation constant
5. Flag groups with multiple uses and no domain separation

### Implementation Effort
**2-3 days** (after #1192, #1194)

---

## Implementation Timeline

### Week 1: Z001 (CRITICAL Priority)
- Day 1-2: State-mutation detection logic
- Day 3-4: Nullifier pattern recognition
- Day 5: Testing and fixtures

### Week 2: Z009 (Resource Exhaustion)
- Day 1-2: Loop analysis and bound checking
- Day 3: Testing and documentation

### Week 3: Z011 (Domain Separation)
- Day 1-2: Cross-function hash pattern analysis
- Day 3: Testing and documentation

### Week 4+: Z007 (Deferred until #1227)
- Blocked on circom parser integration
- 1-2 weeks after parser available

**Total Estimated Effort**: 3-4 weeks (Z001, Z009, Z011), plus 1-2 weeks for Z007 later

---

## Rule Documentation Templates

Each rule needs `docs/rules/Z00X.md` with:

1. **Severity** and **Description**
2. **Vulnerable Pattern** (code example)
3. **Secure Pattern** (code example)
4. **Why This Matters** (attack impact)
5. **Detection Method** (algorithm overview)
6. **Related Rules**
7. **References** (external resources)

---

## Test Fixtures Required

For each rule, create `contracts/fixtures/finding-codes/z00X_*.rs`:

- **Trigger fixture**: Should be flagged by rule
- **Clean fixture**: Should NOT be flagged
- **Snapshot tests**: Verify SARIF output

---

## Success Criteria

### Z001
- [ ] Detects missing nullifier before transfer
- [ ] Does not flag proper nullifier checks
- [ ] Snapshot tests pass
- [ ] Documented in docs/rules/Z001.md

### Z007
- [ ] Parses circom source (after #1227)
- [ ] Detects signals without range checks
- [ ] Snapshot tests with circom fixtures pass

### Z009
- [ ] Detects unbounded proof-verification loops
- [ ] Does not flag bounded loops
- [ ] Snapshot tests pass

### Z011
- [ ] Detects hash reuse without domain separation
- [ ] Cross-function analysis works
- [ ] Snapshot tests pass

---

## Dependencies

- **#1192**: ZK taint-tracking and analysis infrastructure
- **#1194**: Proof-verification pattern detection
- **#1227**: Circom parser (Z007 only)
- **#1217**: Paired fixtures for Z001

---

## Risk Mitigation

### High False Positive Rate
- **Risk**: Rules flag legitimate patterns
- **Mitigation**: Extensive fixture testing, manual review
- **Fallback**: Mark as experimental, tune thresholds

### Parser Integration Complexity (Z007)
- **Risk**: Circom parser integration takes longer than expected
- **Mitigation**: Defer Z007 to separate PR
- **Fallback**: Document as future enhancement

### State Mutation Detection Accuracy (Z001)
- **Risk**: Miss subtle state mutations
- **Mitigation**: Conservative heuristics, flag borderline cases
- **Fallback**: User can suppress false positives

---

**Document Version**: 1.0  
**Next Review**: Upon completion of #1192, #1194
