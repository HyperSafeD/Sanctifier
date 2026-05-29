# ERC-20 Allowance Race Condition

## The Problem

The classic `approve → transferFrom` race condition is well-known on EVM chains.
The Soroban equivalent exists whenever a contract stores a numeric allowance and
exposes a bare `approve(spender, amount)` function.

### Attack sequence

```
1. Alice calls approve(Bob, 100)          → allowance = 100
2. Alice decides to lower it to 50
   and submits approve(Bob, 50)
3. Bob sees the pending tx and front-runs:
   transfer_from(Bob, Alice, ..., 100)    → Bob takes 100, allowance = 0
4. Alice's approve(Bob, 50) lands         → allowance = 50
5. Bob calls transfer_from again for 50   → Bob takes another 50
```

**Total drained: 150 — but Alice intended only 50.**

## Mitigation

Never use a bare `approve` to *change* an existing non-zero allowance.
Use atomic delta helpers instead:

```rust
// ✅ Safe — atomic increase
contract.increase_allowance(&owner, &spender, &delta);

// ✅ Safe — atomic decrease (clamps to zero)
contract.decrease_allowance(&owner, &spender, &delta);

// ❌ Unsafe when current allowance != 0
contract.approve(&owner, &spender, &new_amount);
```

## Reference implementation

See [`contracts/allowance-race/src/lib.rs`](../contracts/allowance-race/src/lib.rs)
for a complete fixture with:

- `approve` — the vulnerable path (for demonstration)
- `transfer_from` — consumes allowance
- `increase_allowance` / `decrease_allowance` — safe atomic helpers
- Tests proving the exploit and the mitigation

## Further reading

- [EIP-20 known attack](https://docs.google.com/document/d/1YLPtQxZu1eV3304M9GnR4bKMBBHFXqoiDFBe5QLHM4/edit)
- [OpenZeppelin increaseAllowance](https://docs.openzeppelin.com/contracts/4.x/api/token/erc20#ERC20-increaseAllowance-address-uint256-)
