# private-transfer

A shielded-balance private transfer demo contract built on Soroban, implementing
a Zcash-style commitment/nullifier pattern with Groth16 proof verification.

## Privacy model

| What is **hidden**                   | What is **visible**                                |
|--------------------------------------|----------------------------------------------------|
| Transfer amount                      | That a shielded operation occurred                 |
| Sender identity                      | Token contract address                             |
| Receiver identity                    | Commitment / nullifier hashes (no preimage)        |

## Operations

### `shield(depositor, amount, commitment, proof)`
Lock `amount` of the backing token into the shielded pool.
A Groth16 proof attests that `commitment` is a well-formed Pedersen commitment
to `(amount, randomness)`.

### `private_transfer(nullifier, new_commitment, proof)`
Spend a note (identified by `nullifier`) and create a new note
(`new_commitment`) without revealing the amount.
The nullifier is burned to prevent double-spend.

### `unshield(recipient, nullifier, amount, proof)`
Burn a note and release `amount` public tokens to `recipient`.

## Limitations (teaching example — not production)

- The Groth16 verifier is a stub (`verify_proof`). Wire in the real on-chain
  verifier from #1216 before deployment.
- The commitment set is a flat mapping, not a Merkle tree. Production Zcash-
  style protocols use an append-only Merkle tree so membership proofs are O(log n).
- The verifying key is stored in mutable contract storage. In production, commit
  the verifying key at deployment time and make it immutable (see finding Z009).
- This example demonstrates the nullifier pattern but does not implement
  shielded Merkle-tree roots — add those for full ZK-SNARK membership proofs.

## Security properties tested

- `shield_rejects_zero_proof` — all-zero proof bytes are rejected.
- `private_transfer_prevents_double_spend` — same nullifier cannot be spent twice.
- `unshield_rejects_spent_nullifier` — unshield with an already-spent nullifier fails.
- `double_initialize_rejected` — contract can only be initialized once.
