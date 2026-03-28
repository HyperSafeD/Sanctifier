---- MODULE Sep41TokenTotalSupply ----
(*
  SEP-41 token template — total supply invariant (two-account abstraction).

  Invariant: totalSupply = a + b.

  Modeled ops:
    - Transfer / TransferFrom: move tokens between a and b.
    - Burn / BurnFrom: reduce a balance and totalSupply equally.
    - Mint (administrative): increase one balance and totalSupply.
    - Approve: metadata only (allowanceAB).

  Model-check: add `SupplyInvariant` (and `TypeOK`) as invariants in TLC.
*)

EXTENDS Integers, TLC

VARIABLES a, b, totalSupply, allowanceAB

vars == << a, b, totalSupply, allowanceAB >>

SupplyInvariant ==
    totalSupply = a + b

TypeOK ==
    (* TLC-friendly bounds; widen if you increase mint/transfer ranges *)
    /\ a \in 0..5000 /\ b \in 0..5000 /\ totalSupply \in 0..5000
    /\ allowanceAB \in 0..500

Init ==
    /\ a = 0
    /\ b = 0
    /\ totalSupply = 0
    /\ allowanceAB = 0

TransferAtoB(amt) ==
    /\ amt \in 1..100
    /\ a >= amt
    /\ a' = a - amt
    /\ b' = b + amt
    /\ totalSupply' = totalSupply
    /\ UNCHANGED allowanceAB

TransferBtoA(amt) ==
    /\ amt \in 1..100
    /\ b >= amt
    /\ b' = b - amt
    /\ a' = a + amt
    /\ totalSupply' = totalSupply
    /\ UNCHANGED allowanceAB

BurnFromA(amt) ==
    /\ amt \in 1..100
    /\ a >= amt
    /\ a' = a - amt
    /\ b' = b
    /\ totalSupply' = totalSupply - amt
    /\ UNCHANGED allowanceAB

BurnFromB(amt) ==
    /\ amt \in 1..100
    /\ b >= amt
    /\ b' = b - amt
    /\ a' = a
    /\ totalSupply' = totalSupply - amt
    /\ UNCHANGED allowanceAB

MintToA(amt) ==
    /\ amt \in 1..100
    /\ a' = a + amt
    /\ b' = b
    /\ totalSupply' = totalSupply + amt
    /\ UNCHANGED allowanceAB

MintToB(amt) ==
    /\ amt \in 1..100
    /\ b' = b + amt
    /\ a' = a
    /\ totalSupply' = totalSupply + amt
    /\ UNCHANGED allowanceAB

Approve(newAllow) ==
    /\ newAllow \in 0..500
    /\ allowanceAB' = newAllow
    /\ UNCHANGED << a, b, totalSupply >>

Next ==
    \/ \E amt \in 1..100 : TransferAtoB(amt)
    \/ \E amt \in 1..100 : TransferBtoA(amt)
    \/ \E amt \in 1..100 : BurnFromA(amt)
    \/ \E amt \in 1..100 : BurnFromB(amt)
    \/ \E amt \in 1..100 : MintToA(amt)
    \/ \E amt \in 1..100 : MintToB(amt)
    \/ \E na \in 0..500 : Approve(na)

Spec == Init /\ [][Next]_vars

=============================================================================
