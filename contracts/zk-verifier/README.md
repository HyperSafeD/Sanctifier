# ZK Verifier — Groth16 Proof-Verifier Reference Contract

A secure, production-quality reference implementation of a Groth16 proof-verifier contract for Soroban, incorporating every mitigation identified by the Sanctifier Z-rule set.

## Security Properties

| Property | Rule | Status |
|----------|------|--------|
| Nullifier double-spend check | Z001 | ✅ `NullifierSet::assert_unspent` called before state mutation |
| Public-input binding | Z003 | ✅ Proof bound to context + nullifier via SHA-256 |
| Verifying-key integrity | Z005 | ✅ SHA-256 hash stored and verified on each call |
| Under-constrained inputs | Z007 | ✅ Structural validation of proof elements |
| VK rotation access control | Z010 | ✅ Multisig quorum + timelock enforced |
| Verification result handling | Z013 | ✅ `Result` propagated, never discarded |

## API

### `initialize(admin: Address, initial_vk: Bytes)`
One-time setup. Stores the verifying key and its integrity hash.

### `verify_proof(proof: Bytes, public_inputs: Vec<BytesN<32>>, context: Bytes, nullifier: Bytes) -> Result<(), VerifierError>`
Core verification flow:
1. Loads the verifying key from storage and checks its integrity hash (Z005).
2. Parses and structurally validates the Groth16 proof.
3. Checks the nullifier for double-spend (Z001) — uses `NullifierSet` for TTL-managed spent tracking.
4. Binds public inputs via SHA-256 (Z003).
5. Delegates to the pairing-check stub (see [Cryptographic Note](#cryptographic-note)).

### `propose_rotation`, `approve_rotation`, `execute_rotation`, `cancel_rotation`
Three-phase VK rotation with multisig + timelock (Z010):
- **Propose**: submits a new VK with a timelock delay.
- **Approve**: collects signer approvals toward a configurable threshold.
- **Execute**: applies the VK only after quorum AND timelock are met.
- **Cancel**: always permitted; prevents execution of a pending rotation.

### `set_threshold(threshold: u32)`
Sets the approval threshold for VK rotation (requires contract auth).

## Cryptographic Note

The actual ate-pairing computation (the core of Groth16 verification) is **not yet executable in Soroban WASM** due to the lack of a native BLS12-381 or BN254 precompile. This contract performs structural validation (zero-element checks, length checks, public-input count matching) and has a well-defined `pairing_check` stub where a real pairing implementation would be plugged in.

When a native WASM pairing shim or Soroban precompile becomes available, the pairing-check function in `groth16.rs` can be replaced with the full verification equation:

```
e(A, B) == e(α, β) · e(∑(pi_i · γ_abc_i), γ) · e(C, δ)
```

## Formal Verification

The VK-rotation pure logic is verified with Kani under `#[cfg(kani)]`:
- No rotation completes without quorum.
- No rotation completes before the timelock elapses.
- Cancel always prevents a pending rotation from completing.

See `vk_storage.rs` for the pure-function proofs.

## Development

```bash
# Run unit tests
cargo test -p zk-verifier

# Run Kani proofs (requires Kani installed)
# cargo kani --package zk-verifier

# Scan with Sanctifier
# sanctifier analyze contracts/zk-verifier
```

## Limitations

- The pairing computation is a stub — real verification requires a WASM pairing implementation.
- Nullifier entries use persistent storage with explicit TTL bumps.
- The multisig signer set is managed externally; this contract tracks only the approval threshold.
