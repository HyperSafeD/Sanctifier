# ZK Security Guide for Soroban Contracts

> **Audience:** Developers building zero-knowledge features on Soroban / Stellar.  
> **Purpose:** Secure design patterns, common pitfalls mapped to Z-rules, and a pre-deployment checklist.  
> **Related:** [Threat Model](adr/), [Individual Z-rule docs](rules/), [Security Checklist](SECURITY-CHECKLIST.md)

---

## Table of Contents

1. [ZK on Soroban — Overview](#1-zk-on-soroban--overview)  
2. [Secure Design Patterns](#2-secure-design-patterns)  
   - 2.1 Nullifiers  
   - 2.2 Commitments and Blinding Factors  
   - 2.3 Public-Input Binding  
   - 2.4 Trusted-Setup Key Management  
   - 2.5 Domain Separation  
   - 2.6 Merkle Membership Proofs  
3. [Z-Rule Catalog](#3-z-rule-catalog)  
4. [Pre-Deployment Security Checklist](#4-pre-deployment-security-checklist)  
5. [Further Reading](#5-further-reading)

---

## 1. ZK on Soroban — Overview

Soroban smart contracts can act as **on-chain ZK verifiers**: they receive a cryptographic proof, verify it against a committed verifying key, and take an action (mint, transfer, vote, update state) if the proof is valid.

This pattern is powerful but introduces an entirely new attack surface beyond traditional smart-contract security. The ZK verifier sits at the intersection of:

- **On-chain authorization logic** — who can call which function, and when?  
- **Cryptographic correctness** — is the proof system instantiated correctly?  
- **Application-layer soundness** — does a valid proof actually mean what the contract thinks it means?

Sanctifier's **Z-rule set** (Z001–Z014) covers the recurring vulnerability classes at this intersection, grounded in real-world audit findings and post-mortems.

### Key principles

| Principle | One-liner |
|-----------|-----------|
| Consume proofs exactly once | Every verified proof must leave a nullifier record (Z001, Z006) |
| Bind proofs to context | Public inputs must commit to the caller, recipient, amount (Z003) |
| Guard the verifying key | Rotation must require admin auth; integrity must be checked before use (Z004, Z005, Z010) |
| Range-check all field elements | Public inputs must be validated before the verifier call (Z007) |
| Separate commitment domains | Never reuse a hash function across distinct commitment types (Z011) |
| Verify against your root | Merkle proofs must check against the contract's committed root (Z014) |

---

## 2. Secure Design Patterns

### 2.1 Nullifiers

A **nullifier** is a unique, deterministic identifier derived from a secret (e.g. the leaf's secret key). It is stored after a proof is consumed to prevent replay.

**Pattern:**

```rust
pub fn claim(env: Env, proof: Vec<u64>, public_inputs: Vec<u64>) {
    let nullifier = public_inputs.get(0).expect("nullifier required");

    // 1. Check — reject if already spent.
    let key = (symbol_short!("null"), nullifier);
    assert!(!env.storage().persistent().has(&key), "proof already consumed");

    // 2. Verify — proof is sound.
    verify_proof(&env, &proof, &public_inputs);

    // 3. Effect — take the action.
    transfer_funds(&env, &public_inputs);

    // 4. Record — mark as spent.
    env.storage().persistent().set(&key, &true);
}
```

**Rules covered:** [Z001](rules/Z001.md), [Z006](rules/Z006.md)

---

### 2.2 Commitments and Blinding Factors

Commitments hide a value while binding the prover to it. The blinding factor must be:

- **Unpredictable** — generated off-chain by a secure RNG, never derived from ledger sequence, timestamp, or other on-chain predictables.
- **Secret** — never included in any event, log, or public output.

**Pattern:**

```rust
// Caller generates blinding factor off-chain (e.g. with `crypto.getRandomValues`)
// and passes it as a private input to the circuit. The contract never sees it.
pub fn commit(env: Env, commitment: BytesN<32>) {
    env.storage().persistent().set(&DataKey::Commitment, &commitment);
}
```

**Rules covered:** [Z002](rules/Z002.md), [Z012](rules/Z012.md)

---

### 2.3 Public-Input Binding

Every ZK-verifier entry point must include transaction-specific context in the public inputs: caller, recipient, amount, and contract ID. This prevents an attacker from taking a valid proof and redirecting its effect.

**Pattern:**

```rust
pub fn withdraw(env: Env, proof: Vec<u64>, recipient: Address, amount: i128) {
    // Validate range before including in public inputs (Z007).
    assert!(amount > 0 && amount <= MAX_WITHDRAW, "amount out of range");

    // Bind to this transaction's context.
    let recipient_hash = hash_address(&env, &recipient);
    let contract_id_hash = hash_address(&env, &env.current_contract_address());
    let public_inputs = vec![&env, recipient_hash, amount as u64, contract_id_hash];

    verify_proof(&env, &proof, &public_inputs);
    token_client.transfer(&env.current_contract_address(), &recipient, &amount);
}
```

**Rules covered:** [Z003](rules/Z003.md), [Z007](rules/Z007.md)

---

### 2.4 Trusted-Setup Key Management

The verifying key is the cryptographic root of trust. Treat it like an admin key:

1. **Never hardcode it** — store in governance-controlled persistent storage.  
2. **Protect rotation** — require admin `require_auth()` before any update.  
3. **Verify integrity before use** — hash-check the loaded key on every verification call.

**Pattern:**

```rust
pub fn set_verifying_key(env: Env, vk: BytesN<64>) {
    let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    let hash = env.crypto().sha256(&Bytes::from_slice(&env, vk.as_ref()));
    env.storage().persistent().set(&DataKey::VerifyingKey, &vk);
    env.storage().persistent().set(&DataKey::VkHash, &hash);
}

pub fn verify(env: Env, proof: Vec<u8>, inputs: Vec<u64>) -> bool {
    let vk: BytesN<64> = env.storage().persistent().get(&DataKey::VerifyingKey).unwrap();
    let expected: BytesN<32> = env.storage().persistent().get(&DataKey::VkHash).unwrap();
    assert_eq!(env.crypto().sha256(&Bytes::from_slice(&env, vk.as_ref())), expected);
    groth16_verify(vk.as_ref(), &proof, &inputs)
}
```

**Rules covered:** [Z004](rules/Z004.md), [Z005](rules/Z005.md), [Z010](rules/Z010.md)

---

### 2.5 Domain Separation

Always use a distinct, versioned domain tag when hashing for different commitment types. Prefix every hash call with a unique byte string that identifies the context.

```rust
fn commit_with_domain(env: &Env, domain: &[u8], value: &[u8]) -> BytesN<32> {
    let mut input = Bytes::new(env);
    input.extend_from_slice(domain);
    input.extend_from_slice(value);
    env.crypto().sha256(&input)
}

// Example usage
let nullifier_hash  = commit_with_domain(&env, b"sanctifier:nullifier:v1",  secret.as_ref());
let amount_hash     = commit_with_domain(&env, b"sanctifier:amount:v1",     &amount.to_be_bytes());
let recipient_hash  = commit_with_domain(&env, b"sanctifier:recipient:v1",  recipient_bytes.as_ref());
```

**Rules covered:** [Z011](rules/Z011.md)

---

### 2.6 Merkle Membership Proofs

When using Merkle trees for on-chain set commitments (whitelists, UTXOs, anonymity sets):

- **Load the root from contract storage** — never accept a caller-supplied root.
- **Verify the proof** before taking any action.
- **Nullify the leaf** after use to prevent double-claim (combine with Z001 pattern).

```rust
pub fn claim(env: Env, leaf: BytesN<32>, path: Vec<BytesN<32>>) {
    let root: BytesN<32> = env.storage().persistent().get(&DataKey::MerkleRoot).unwrap();
    assert!(verify_merkle_proof(&leaf, &path, &root), "not a member");

    let spent_key = (symbol_short!("spent"), leaf.clone());
    assert!(!env.storage().persistent().has(&spent_key), "already claimed");
    env.storage().persistent().set(&spent_key, &true);

    transfer_reward(&env);
}
```

**Rules covered:** [Z014](rules/Z014.md)

---

## 3. Z-Rule Catalog

| Rule | Name | Severity | Category |
|------|------|----------|----------|
| [Z001](rules/Z001.md) | Missing Nullifier / Double-Spend Check | Critical | zk-proof-integrity |
| [Z002](rules/Z002.md) | Insecure or Predictable Randomness as Circuit Input | High | zk-randomness |
| [Z003](rules/Z003.md) | Missing Public-Input Binding (Proof Malleability) | Critical | zk-proof-integrity |
| [Z004](rules/Z004.md) | Hardcoded Trusted-Setup Parameters | Critical | zk-trusted-setup |
| [Z005](rules/Z005.md) | Missing Verifying-Key Integrity Check | High | zk-trusted-setup |
| [Z006](rules/Z006.md) | Missing Proof Nonce / Uniqueness Enforcement | High | zk-proof-integrity |
| [Z007](rules/Z007.md) | Under-Constrained Circuit Inputs | Critical | zk-circuit-constraints |
| [Z008](rules/Z008.md) | Curve / Field Mismatch | Critical | zk-circuit-constraints |
| [Z009](rules/Z009.md) | Unbounded Proof-Verification Loop | High | zk-resource |
| [Z010](rules/Z010.md) | Missing Access Control on Verifying-Key Rotation | Critical | zk-access-control |
| [Z011](rules/Z011.md) | Commitment Reuse Without Domain Separation | High | zk-cryptography |
| [Z012](rules/Z012.md) | ZK Property Leak via Public-Output Over-Exposure | Medium | zk-privacy |
| [Z013](rules/Z013.md) | Insufficient Batch-Validation in ZK-Rollup Transitions | Critical | zk-proof-integrity |
| [Z014](rules/Z014.md) | Missing Merkle-Root Inclusion-Proof Verification | Critical | zk-proof-integrity |

**Critical rules** (Z001, Z003, Z004, Z007, Z008, Z010, Z013, Z014) must be resolved before mainnet deployment. **High rules** should be resolved before public testnet exposure.

---

## 4. Pre-Deployment Security Checklist

### Proof lifecycle

- [ ] Every proof-consuming entry point records a nullifier before transferring value (Z001)
- [ ] Proofs are bound to an epoch or nonce to prevent cross-period replay (Z006)
- [ ] Public inputs include recipient, amount, and contract ID (Z003)
- [ ] All public input field elements are range-validated before the verifier call (Z007)

### Trusted setup

- [ ] Verifying key is stored in governance-controlled persistent storage, not hardcoded (Z004)
- [ ] Verifying-key rotation is protected by `require_auth()` on an admin/multisig (Z010)
- [ ] Verifying-key integrity is hash-checked before every use (Z005)
- [ ] Curve/field identifier is validated at verifier entry (Z008)

### Cryptographic hygiene

- [ ] Blinding factors are generated off-chain by a secure RNG — no on-chain entropy (Z002)
- [ ] All commitment types use distinct, versioned domain separation tags (Z011)
- [ ] Public outputs do not include private witness values or full input vectors (Z012)

### State integrity

- [ ] Merkle inclusion proofs are verified against the contract's committed root, not a caller-supplied root (Z014)
- [ ] Batch/rollup proofs assert `old_root == current_root` before accepting a state transition (Z013)
- [ ] Batch-proof loops are bounded by a protocol constant (Z009)

### General (S-rules still apply)

- [ ] All privileged functions have `require_auth()` guards (S001)
- [ ] Arithmetic operations are overflow-safe (S002)
- [ ] Storage keys avoid collisions (S004)
- [ ] Contract is protected against re-initialization (S008)

---

## 5. Further Reading

- [0xPARC ZK Bug Tracker](https://github.com/0xPARC/zk-bug-tracker) — curated list of real-world ZK application bugs
- [Trail of Bits: ZK Security](https://blog.trailofbits.com/2022/04/13/part-1-coordinated-disclosure-of-vulnerabilities-affecting-girault-bulletproofs-and-plonk/)
- [Zellic: Common ZK vulnerabilities](https://www.zellic.io/blog/the-frozen-heart-vulnerability-in-plonk)
- [Noir language documentation](https://noir-lang.org/)
- [Soroban authorization model](https://soroban.stellar.org/docs/fundamentals/authorization)
- [Soroban fees and metering](https://soroban.stellar.org/docs/fundamentals/fees-and-metering)
- [Sanctifier S-rule documentation](rules/) — general Soroban security rules
- [Sanctifier Security Checklist](SECURITY-CHECKLIST.md)
