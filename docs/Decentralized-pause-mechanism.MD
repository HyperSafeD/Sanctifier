# Decentralized Pause Mechanism — Reference Pattern

> A battle-tested architectural pattern for halting smart contract execution during emergencies, without centralizing control in a single account.

---

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Design Goals](#design-goals)
3. [Architecture Overview](#architecture-overview)
4. [Core Pattern: Multi-Sig Guardian Council](#core-pattern-multi-sig-guardian-council)
5. [Pattern Variants](#pattern-variants)
6. [Reference Implementation (Solidity)](#reference-implementation-solidity)
7. [Integration Guide](#integration-guide)
8. [Security Considerations](#security-considerations)
9. [Testing Checklist](#testing-checklist)
10. [Known Trade-offs](#known-trade-offs)

---

## Problem Statement

Smart contracts managing significant value need an emergency stop mechanism. The naive approach — a single `owner` who can pause — creates critical centralization risk:

- If the owner's private key is compromised, an attacker can prevent legitimate users from withdrawing funds.
- A single point of failure contradicts the decentralization ethos of Web3.
- Users cannot independently verify that the pause power is being used fairly.

**The challenge:** Allow rapid emergency response _without_ concentrating power in one address.

---

## Design Goals

| Goal                      | Description                                                          |
| ------------------------- | -------------------------------------------------------------------- |
| **Decentralized control** | No single address can pause unilaterally                             |
| **Speed**                 | Pause can be triggered quickly enough to be useful during an exploit |
| **Transparency**          | All pause/unpause actions are on-chain and auditable                 |
| **Recoverability**        | The system can be unpaused through a defined process                 |
| **Resilience**            | Compromise of one guardian key does not compromise the mechanism     |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Protected Contract                    │
│                                                         │
│   modifier whenNotPaused { ... }                        │
│   reads isPaused from ──────────────────────────┐       │
└─────────────────────────────────────────────────│───────┘
                                                  │
                                                  ▼
┌─────────────────────────────────────────────────────────┐
│                  PauseController Contract               │
│                                                         │
│   bool public paused                                    │
│   mapping(address => bool) public isGuardian            │
│   uint256 public pauseThreshold   (e.g. 2-of-5)        │
│   mapping(address => bool) public hasCastVote           │
│   uint256 public pauseVotes                             │
│                                                         │
│   castPauseVote()    ◄──── Guardian A                  │
│   castPauseVote()    ◄──── Guardian B    → PAUSED ✓    │
│   revokeVote()                                          │
│   unpause()          ◄──── Timelock / DAO              │
└─────────────────────────────────────────────────────────┘
          ▲          ▲          ▲
     Guardian 1  Guardian 2  Guardian 3
    (Protocol   (Security   (Community
      Team)       Firm)      Multisig)
```

**Key insight:** The pause _triggers_ with M-of-N guardian votes (fast), but unpause _requires_ a slower governance process (deliberate). This asymmetry is intentional — it's easier to stop than to restart.

---

## Core Pattern: Multi-Sig Guardian Council

### Roles

| Role                 | Responsibility                              | Recommended Composition                                           |
| -------------------- | ------------------------------------------- | ----------------------------------------------------------------- |
| **Guardian**         | Cast pause votes during emergencies         | Protocol team multisig, auditor, community elected representative |
| **Guardian Council** | The set of all guardians (M-of-N)           | 3-of-5 or 2-of-3 for small protocols; 4-of-7 for large ones       |
| **Governance / DAO** | Authorize unpause after root cause analysis | Token holders via on-chain vote or Timelock                       |
| **Admin**            | Add/remove guardians                        | Should itself be a governance-controlled address                  |

### Pause Flow

```
[Exploit Detected]
       │
       ▼
Guardian A calls castPauseVote()   → votes: 1 / threshold
       │
       ▼
Guardian B calls castPauseVote()   → votes: 2 / threshold
       │
       ▼
  threshold reached?
       │ YES
       ▼
  paused = true                    → all protected functions revert
       │
       ▼
[Incident Response begins]
       │
       ▼
[Root cause fixed, governance vote passes]
       │
       ▼
  unpause() called by Timelock     → paused = false, votes reset
```

---

## Pattern Variants

### Variant A — Optimistic Pause (Recommended for Speed)

Any **single** guardian can pause, but any other guardian can veto within a short window (e.g., 1 hour). After the window, the pause is permanent until governance unpauses.

- ✅ Fastest response time
- ⚠️ Higher risk if a guardian key is compromised

### Variant B — Threshold Vote (Recommended for Balance)

M-of-N guardians must vote before the contract pauses. Votes accumulate and the pause triggers automatically when the threshold is crossed.

- ✅ Balanced speed vs. safety
- ✅ Resistant to single key compromise
- ⚠️ Coordination required (but guardians should be on-call)

### Variant C — Timelocked Pause with Override

Pause goes into effect after a short delay (e.g., 15 minutes) unless cancelled. A supermajority of guardians can skip the delay in a critical emergency.

- ✅ Allows users to exit before a controversial pause
- ⚠️ Too slow for active exploits

### Variant D — On-Chain Anomaly Trigger

An oracle or monitoring contract (e.g., watching TVL drop or abnormal withdrawal rates) automatically votes to pause when a threshold anomaly is detected.

- ✅ Fully automated, no human latency
- ⚠️ Complex; oracle manipulation is a new attack surface

> **Recommendation for most protocols:** Start with **Variant B (Threshold Vote)** using a 2-of-3 or 3-of-5 council. Upgrade to hybrid patterns as the protocol matures.

---

## Reference Implementation (Solidity)

### 1. `PauseController.sol` — The Pause Authority

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title PauseController
 * @notice Decentralized M-of-N guardian pause mechanism.
 *         Guardians vote to pause; governance is required to unpause.
 * @dev Deploy this separately and point protected contracts at it.
 */
contract PauseController {

    // ─── State ────────────────────────────────────────────────────────────────

    bool public paused;

    address public governance;          // DAO / Timelock that can unpause
    uint256 public pauseThreshold;      // M votes needed to trigger pause

    mapping(address => bool) public isGuardian;
    mapping(address => bool) public hasCastVote;
    uint256 public pauseVoteCount;

    address[] private _guardians;       // Enumerable list for transparency

    // ─── Events ───────────────────────────────────────────────────────────────

    event PauseVoteCast(address indexed guardian, uint256 currentVotes, uint256 threshold);
    event PauseVoteRevoked(address indexed guardian, uint256 currentVotes);
    event ContractPaused(address indexed triggeredBy);
    event ContractUnpaused(address indexed by);
    event GuardianAdded(address indexed guardian);
    event GuardianRemoved(address indexed guardian);
    event ThresholdUpdated(uint256 oldThreshold, uint256 newThreshold);
    event GovernanceUpdated(address indexed oldGov, address indexed newGov);

    // ─── Modifiers ────────────────────────────────────────────────────────────

    modifier onlyGuardian() {
        require(isGuardian[msg.sender], "PauseController: caller is not a guardian");
        _;
    }

    modifier onlyGovernance() {
        require(msg.sender == governance, "PauseController: caller is not governance");
        _;
    }

    modifier notPaused() {
        require(!paused, "PauseController: already paused");
        _;
    }

    modifier isPaused() {
        require(paused, "PauseController: not paused");
        _;
    }

    // ─── Constructor ──────────────────────────────────────────────────────────

    /**
     * @param _guardians      Initial set of guardian addresses
     * @param _threshold      Number of guardian votes required to pause (M-of-N)
     * @param _governance     Address authorized to unpause (e.g., Timelock)
     */
    constructor(
        address[] memory _guardiansInit,
        uint256 _threshold,
        address _governance
    ) {
        require(_guardiansInit.length >= _threshold, "PauseController: threshold exceeds guardian count");
        require(_threshold > 0, "PauseController: threshold must be > 0");
        require(_governance != address(0), "PauseController: zero address governance");

        governance = _governance;
        pauseThreshold = _threshold;

        for (uint256 i = 0; i < _guardiansInit.length; i++) {
            _addGuardian(_guardiansInit[i]);
        }
    }

    // ─── Guardian Actions ─────────────────────────────────────────────────────

    /**
     * @notice Cast a vote to pause all protected contracts.
     *         Pause triggers automatically when threshold is reached.
     */
    function castPauseVote() external onlyGuardian notPaused {
        require(!hasCastVote[msg.sender], "PauseController: vote already cast");

        hasCastVote[msg.sender] = true;
        pauseVoteCount++;

        emit PauseVoteCast(msg.sender, pauseVoteCount, pauseThreshold);

        if (pauseVoteCount >= pauseThreshold) {
            paused = true;
            emit ContractPaused(msg.sender);
        }
    }

    /**
     * @notice Revoke a previously cast pause vote.
     *         Only valid before the threshold has been reached.
     */
    function revokePauseVote() external onlyGuardian notPaused {
        require(hasCastVote[msg.sender], "PauseController: no vote to revoke");

        hasCastVote[msg.sender] = false;
        pauseVoteCount--;

        emit PauseVoteRevoked(msg.sender, pauseVoteCount);
    }

    // ─── Governance Actions ───────────────────────────────────────────────────

    /**
     * @notice Unpause all protected contracts.
     *         Resets all guardian votes. Callable only by governance.
     * @dev    Governance should require a full post-mortem before calling this.
     */
    function unpause() external onlyGovernance isPaused {
        paused = false;
        _resetVotes();
        emit ContractUnpaused(msg.sender);
    }

    /**
     * @notice Add a new guardian to the council.
     */
    function addGuardian(address guardian) external onlyGovernance {
        _addGuardian(guardian);
    }

    /**
     * @notice Remove a guardian from the council.
     */
    function removeGuardian(address guardian) external onlyGovernance {
        require(isGuardian[guardian], "PauseController: not a guardian");
        require(
            _guardians.length - 1 >= pauseThreshold,
            "PauseController: would drop below threshold"
        );

        isGuardian[guardian] = false;

        // Remove from enumerable array
        for (uint256 i = 0; i < _guardians.length; i++) {
            if (_guardians[i] == guardian) {
                _guardians[i] = _guardians[_guardians.length - 1];
                _guardians.pop();
                break;
            }
        }

        // Deduct vote if this guardian had voted
        if (hasCastVote[guardian]) {
            hasCastVote[guardian] = false;
            pauseVoteCount--;
        }

        emit GuardianRemoved(guardian);
    }

    /**
     * @notice Update the pause threshold. New threshold must not exceed current guardian count.
     */
    function setThreshold(uint256 newThreshold) external onlyGovernance {
        require(newThreshold > 0, "PauseController: threshold must be > 0");
        require(newThreshold <= _guardians.length, "PauseController: threshold exceeds guardian count");
        emit ThresholdUpdated(pauseThreshold, newThreshold);
        pauseThreshold = newThreshold;
    }

    /**
     * @notice Transfer governance to a new address (e.g., upgraded Timelock).
     */
    function setGovernance(address newGovernance) external onlyGovernance {
        require(newGovernance != address(0), "PauseController: zero address");
        emit GovernanceUpdated(governance, newGovernance);
        governance = newGovernance;
    }

    // ─── View Functions ───────────────────────────────────────────────────────

    /**
     * @notice Returns all current guardian addresses.
     */
    function getGuardians() external view returns (address[] memory) {
        return _guardians;
    }

    /**
     * @notice Returns how many more votes are needed to trigger a pause.
     */
    function votesRemaining() external view returns (uint256) {
        if (pauseVoteCount >= pauseThreshold) return 0;
        return pauseThreshold - pauseVoteCount;
    }

    // ─── Internal Helpers ─────────────────────────────────────────────────────

    function _addGuardian(address guardian) internal {
        require(guardian != address(0), "PauseController: zero address");
        require(!isGuardian[guardian], "PauseController: already a guardian");

        isGuardian[guardian] = true;
        _guardians.push(guardian);

        emit GuardianAdded(guardian);
    }

    function _resetVotes() internal {
        for (uint256 i = 0; i < _guardians.length; i++) {
            hasCastVote[_guardians[i]] = false;
        }
        pauseVoteCount = 0;
    }
}
```

---

### 2. `Pausable.sol` — Mixin for Protected Contracts

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PauseController.sol";

/**
 * @title Pausable
 * @notice Mixin that reads pause state from an external PauseController.
 *         Inherit this in any contract that should be halted during emergencies.
 */
abstract contract Pausable {

    PauseController public pauseController;

    event PauseControllerUpdated(address indexed oldController, address indexed newController);

    constructor(address _pauseController) {
        require(_pauseController != address(0), "Pausable: zero address controller");
        pauseController = PauseController(_pauseController);
    }

    modifier whenNotPaused() {
        require(!pauseController.paused(), "Pausable: contract is paused");
        _;
    }

    modifier whenPaused() {
        require(pauseController.paused(), "Pausable: contract is not paused");
        _;
    }

    /**
     * @dev Override in the child contract; restrict with onlyGovernance or similar.
     */
    function _setPauseController(address newController) internal {
        require(newController != address(0), "Pausable: zero address");
        emit PauseControllerUpdated(address(pauseController), newController);
        pauseController = PauseController(newController);
    }
}
```

---

### 3. `ProtectedVault.sol` — Example Integration

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./Pausable.sol";

/**
 * @title ProtectedVault
 * @notice Example contract using the decentralized pause mechanism.
 */
contract ProtectedVault is Pausable {

    mapping(address => uint256) public balances;

    constructor(address _pauseController) Pausable(_pauseController) {}

    /// @notice Deposit ETH. Blocked when paused.
    function deposit() external payable whenNotPaused {
        balances[msg.sender] += msg.value;
    }

    /// @notice Withdraw ETH. Blocked when paused to prevent drain during exploit.
    function withdraw(uint256 amount) external whenNotPaused {
        require(balances[msg.sender] >= amount, "Vault: insufficient balance");
        balances[msg.sender] -= amount;
        (bool ok, ) = msg.sender.call{value: amount}("");
        require(ok, "Vault: transfer failed");
    }

    /// @notice Emergency withdraw — available ONLY when paused.
    ///         Allows users to exit after governance has verified it is safe.
    function emergencyWithdraw() external whenPaused {
        uint256 amount = balances[msg.sender];
        require(amount > 0, "Vault: nothing to withdraw");
        balances[msg.sender] = 0;
        (bool ok, ) = msg.sender.call{value: amount}("");
        require(ok, "Vault: transfer failed");
    }
}
```

> **Note on `emergencyWithdraw`:** Whether to allow withdrawals during a pause is a design decision. If the exploit _is_ a drain, you may want to block all outflows. If the exploit is an entry-point attack (e.g., a bad deposit path), allowing withdrawals during the pause is user-friendly. Decide per-protocol.

---

## Integration Guide

### Deploying the System

```bash
# Step 1 — Deploy governance (Timelock or multisig)
GOVERNANCE=<timelock_address>

# Step 2 — Deploy PauseController
PauseController.deploy(
  guardians: [GUARDIAN_A, GUARDIAN_B, GUARDIAN_C, GUARDIAN_D, GUARDIAN_E],
  threshold: 3,        # 3-of-5 required
  governance: GOVERNANCE
)

# Step 3 — Deploy protected contracts, passing PauseController address
ProtectedVault.deploy(pauseController: PAUSE_CONTROLLER_ADDRESS)

# Step 4 — Verify all addresses on-chain and document in your audit
```

### Adding `whenNotPaused` to Existing Functions

Apply the modifier to any state-changing function that should halt:

```solidity
// ✅ Apply to value-moving functions
function transfer(...) external whenNotPaused { ... }
function borrow(...) external whenNotPaused { ... }
function swap(...) external whenNotPaused { ... }

// ❌ Do NOT apply to pure view/read functions
function balanceOf(...) external view returns (uint256) { ... }

// ❌ Do NOT apply to emergency recovery functions
function emergencyWithdraw() external whenPaused { ... }
```

---

## Security Considerations

### Guardian Key Management

- Each guardian **must** use a hardware wallet or a multisig (e.g., Gnosis Safe) as their guardian address — never a plain EOA.
- Rotate guardian keys periodically and after any team member departure.
- Guardian addresses should be public and documented so the community can verify the council composition.

### Preventing Griefing

A malicious or compromised guardian voting to pause unnecessarily is a risk. Mitigations:

- Use a threshold ≥ 2 so one actor cannot pause alone.
- Add a time-lock on the _pause itself_ (Variant C) for non-critical pauses.
- Implement guardian slashing or removal in governance for bad-faith pauses.

### Reentrancy During Pause Check

The `whenNotPaused` modifier reads from an external contract (`PauseController`). Ensure this external call cannot be manipulated:

- `PauseController` should be simple with no callbacks.
- Use `view` reads — no state changes occur during the modifier check.

### Upgrade Safety

If your contracts are upgradeable (proxy pattern):

- The `PauseController` address should itself be part of the proxy's storage, upgradeable by governance.
- Ensure upgrades cannot be executed while paused — add `whenNotPaused` to upgrade functions.

### Unpause Governance Requirements (Recommended)

Before governance calls `unpause()`, enforce off-chain (and optionally on-chain) requirements:

1. Root cause of the emergency has been identified.
2. A fix has been deployed and audited (if needed).
3. A post-mortem has been published.
4. On-chain vote has passed with quorum.

---

## Testing Checklist

```
Unit Tests — PauseController
  [ ] Guardian can cast pause vote
  [ ] Non-guardian cannot cast pause vote
  [ ] Duplicate votes from same guardian are rejected
  [ ] Pause triggers exactly when threshold is reached (not before)
  [ ] Guardian can revoke vote before pause
  [ ] Vote revocation after pause is blocked
  [ ] Governance can unpause
  [ ] Non-governance cannot unpause
  [ ] Governance can add/remove guardians
  [ ] Removing a guardian updates vote count correctly
  [ ] Threshold cannot exceed guardian count
  [ ] Events are emitted correctly for all state transitions

Unit Tests — Pausable Contracts
  [ ] Protected functions revert when paused
  [ ] Protected functions succeed when unpaused
  [ ] emergencyWithdraw (if present) works only when paused
  [ ] View/read functions work regardless of pause state

Integration Tests
  [ ] Full pause flow: guardian votes → threshold → paused state
  [ ] Full unpause flow: governance vote → unpaused → votes reset
  [ ] Guardian compromise scenario: 1 of 3 guardian keys lost, pause still achievable
  [ ] Governance compromise scenario: governance cannot pause by itself

Invariant / Fuzz Tests
  [ ] pauseVoteCount never exceeds number of guardians
  [ ] paused is always true iff pauseVoteCount >= threshold at trigger time
  [ ] Total ETH in vault is conserved across pause/unpause cycles
```

---

## Known Trade-offs

| Trade-off                       | Description                                   | Mitigation                                                                |
| ------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------- |
| **Speed vs. Safety**            | Higher threshold = slower pause               | Keep guardians on-call; use alerting tools (Forta, OpenZeppelin Defender) |
| **Guardian liveness**           | Guardians must be reachable 24/7              | Use automated monitoring bots as one guardian slot                        |
| **Governance delay on unpause** | Timelocks slow recovery                       | Pre-agree on fast-track governance process for post-exploits              |
| **External dependency**         | Protected contracts depend on PauseController | Audit PauseController independently; it is a critical path contract       |
| **Pause griefing**              | Coordinated guardians can DoS the protocol    | Guardian selection, slashing mechanisms, community transparency           |

---

## References & Further Reading

- [OpenZeppelin Pausable (centralized baseline)](https://docs.openzeppelin.com/contracts/4.x/api/security#Pausable)
- [Circuit Breakers in DeFi — Trail of Bits](https://blog.trailofbits.com)
- [MakerDAO Emergency Shutdown Module](https://docs.makerdao.com/smart-contract-modules/shutdown)
- [Compound Guardian Pattern](https://compound.finance/docs/governance)
- [Gnosis Safe Multisig](https://safe.global) — recommended for guardian addresses

---

_Pattern version: 1.0 | Solidity: ^0.8.20 | Audited: pending — do not deploy to mainnet without independent audit_
