# ZK Rules Advanced: Implementation Plan

**Status**: Implementation Framework  
**Author**: chonilius  
**Date**: 2026-07-27  
**Issues Addressed**: #1204 (Z008), #1208 (Z012), #1209 (Z013), #1210 (Z014)

---

## Executive Summary

Four advanced ZK detection rules focusing on cross-file analysis, privacy leakage, rollup patterns, and merkle proofs.

**Priority**: Z014 (merkle) → Z013 (rollup) → Z012 (privacy) → Z008 (curve mismatch)

---

## Issue #1210: Z014 - Missing Merkle-Root Verification

### Goal
Detect merkle-path verification where computed root never compared against stored root, or comparison happens after leaf is trusted.

### Vulnerable Pattern
```rust
// ❌ Missing root comparison
pub fn claim_with_merkle(env: Env, leaf: BytesN<32>, path: Vec<BytesN<32>>) {
    let computed_root = compute_merkle_root(&env, leaf, path);
    // ❌ Never checks: computed_root == stored_root
    
    process_claim(&env, &leaf); // Trust leaf without verification!
}
```

### Secure Pattern
```rust
// ✅ Root comparison before trust
pub fn claim_with_merkle(env: Env, leaf: BytesN<32>, path: Vec<BytesN<32>>) {
    let computed_root = compute_merkle_root(&env, leaf, path);
    let stored_root = env.storage().get(&ROOT_KEY).unwrap();
    
    if computed_root != stored_root {
        panic!("Invalid merkle proof");
    }
    
    // Now safe to trust leaf
    process_claim(&env, &leaf);
}
```

### Implementation Effort
**3-4 days** (after #1192, #1194)

---

## Issue #1209: Z013 - Missing Batch-Root Validation (Rollup)

### Goal
Detect ZK-rollup state transitions accepting `(old_root, new_root, proof)` without verifying `old_root == current_stored_root`.

### Vulnerable Pattern
```rust
// ❌ No old-root validation
pub fn apply_batch(env: Env, old_root: BytesN<32>, new_root: BytesN<32>, proof: Proof) {
    verify_zk_proof(&env, proof, &[old_root, new_root]);
    
    // ❌ Never checks: old_root == get_current_root()
    set_root(&env, new_root); // State manipulation possible!
}
```

### Secure Pattern
```rust
// ✅ Old-root validation
pub fn apply_batch(env: Env, old_root: BytesN<32>, new_root: BytesN<32>, proof: Proof) {
    let current_root = get_current_root(&env);
    
    if old_root != current_root {
        panic!("Old root mismatch - invalid state transition");
    }
    
    verify_zk_proof(&env, proof, &[old_root, new_root]);
    set_root(&env, new_root); // Safe state transition
}
```

### Implementation Effort
**3-4 days** (after #1192, #1194)

---

## Issue #1208: Z012 - Public-Output Over-Exposure

### Goal
Detect circuits exposing more public outputs than necessary, potentially leaking private information.

### Vulnerable Pattern
```rust
// ❌ Over-broad public outputs
pub fn verify_age_proof(env: Env, proof: Proof, public_inputs: Vec<u64>) {
    // Public inputs: [age, birthdate, ssn_last4, is_over_21]
    // ❌ Exposes age, birthdate, SSN - only need is_over_21!
    
    verify_zk_proof(&env, proof, &public_inputs);
    
    let is_over_21 = public_inputs[3];
    if is_over_21 != 1 {
        panic!("Age verification failed");
    }
}
```

### Secure Pattern
```rust
// ✅ Minimal public outputs
pub fn verify_age_proof(env: Env, proof: Proof, is_over_21: u64) {
    // Public inputs: [is_over_21] only
    // ✅ Private: age, birthdate, SSN stay in circuit
    
    verify_zk_proof(&env, proof, &[is_over_21]);
    
    if is_over_21 != 1 {
        panic!("Age verification failed");
    }
}
```

### Detection (Heuristic)
- Count public inputs vs expected minimal set
- Flag if > expected (advisory, not hard block)
- Requires manual review confirmation

### Implementation Effort
**3-4 days** (after #1192, #1194)

---

## Issue #1204: Z008 - Curve/Field Mismatch

### Goal
Cross-check elliptic curve parameters between on-chain verifier and off-chain circuit, flag mismatches (e.g., BN254 verifier with BLS12-381 circuit).

### Vulnerable Pattern
```rust
// Circuit: uses BLS12-381 (circom default)
// Verifier contract: configured for BN254
pub fn verify(env: Env, proof: Proof) {
    // ❌ Curve mismatch - proof unverifiable or exploitable
    bn254_verify(&env, proof); // Wrong curve!
}
```

### Detection (Cross-File)
1. Extract curve from circuit source (circom/Noir config)
2. Extract curve from verifier contract type parameters
3. Compare curves across project
4. Flag mismatches

### Dependencies
- **#1227**: Circom parser
- **#1228**: Noir parser  
- **#1229**: Arkworks config parser
- **#1192, #1194**: ZK infrastructure

### Implementation Effort
**4-5 days** (after all parser dependencies)

---

## Implementation Timeline

**Week 1**: Z014 (merkle) + Z013 (rollup)
**Week 2**: Z012 (privacy heuristic)
**Week 3+**: Z008 (deferred until parsers ready)

**Total**: 2-3 weeks for Z014/Z013/Z012, plus 1 week for Z008 later

---

## Dependencies

- **#1192, #1194**: All rules
- **#1227, #1228, #1229**: Z008 only (cross-file parsing)
- **#1220**: Rollup fixture for Z013
- **#1221**: Merkle fixture for Z014

---

## Success Criteria

### Z014
- [ ] Detects missing merkle root comparison
- [ ] Does not flag correct comparison
- [ ] Snapshot tests pass

### Z013
- [ ] Detects missing old-root validation
- [ ] Rollup pattern recognized
- [ ] Snapshot tests pass

### Z012
- [ ] Flags over-broad public inputs (heuristic)
- [ ] Advisory message clear
- [ ] Snapshot tests pass

### Z008
- [ ] Cross-file curve extraction works
- [ ] Mismatch detection accurate
- [ ] Snapshot tests pass

---

**Document Version**: 1.0  
**Next Review**: Upon #1192, #1194 completion
