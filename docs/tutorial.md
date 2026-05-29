# 🛡️ Sanctifier Tutorial: Fix the Broken Contract

Welcome to the Sanctifier interactive tutorial. In this guide, you'll
audit a deliberately vulnerable smart contract, identify the security
flaws, and apply fixes using Sanctifier's analysis tools.

By the end you will:
- Identify 3 common Solidity vulnerabilities
- Use Sanctifier to surface them automatically
- Apply and verify the fixes

---

## Prerequisites

- Node.js ≥ 18
- Sanctifier CLI installed (`npm install -g @hypersafed/sanctifier`)
- Basic Solidity knowledge

---

## Step 1 — Clone the Tutorial Contract

Create a new file `contracts/VulnerableVault.sol` with the following
code. **Do not fix anything yet — the bugs are intentional.**
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title VulnerableVault
/// @notice DO NOT USE IN PRODUCTION — intentionally broken for tutorial purposes
contract VulnerableVault {
    mapping(address => uint256) public balances;
    address public owner;

    constructor() {
        owner = msg.sender;
    }

    // 🐛 Bug 1: No reentrancy guard
    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount, "Insufficient balance");
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success, "Transfer failed");
        balances[msg.sender] -= amount; // state updated AFTER external call
    }

    // 🐛 Bug 2: Anyone can call this — missing access control
    function emergencyDrain() external {
        payable(msg.sender).transfer(address(this).balance);
    }

    // 🐛 Bug 3: Timestamp dependence — miners can manipulate block.timestamp
    function isWithdrawalWindow() public view returns (bool) {
        return block.timestamp % 2 == 0;
    }

    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }
}
```

---

## Step 2 — Run Sanctifier

Point Sanctifier at the contract:
```bash
sanctifier analyze contracts/VulnerableVault.sol
```

You should see output similar to:
```
[CRITICAL] Reentrancy: withdraw() — state mutated after external call (line 17)
[CRITICAL] Missing access control: emergencyDrain() — no onlyOwner modifier (line 23)
[MEDIUM]   Timestamp dependence: isWithdrawalWindow() — block.timestamp is miner-manipulable (line 28)

3 issues found. 0 passed.
```

---

## Step 3 — Fix the Bugs

Now apply the fixes one at a time.

### Fix 1 — Reentrancy (Checks-Effects-Interactions)

Move the state update **before** the external call:
```solidity
function withdraw(uint256 amount) external {
    require(balances[msg.sender] >= amount, "Insufficient balance");
    balances[msg.sender] -= amount; // ✅ state updated BEFORE external call
    (bool success, ) = msg.sender.call{value: amount}("");
    require(success, "Transfer failed");
}
```

> **Why?** A malicious contract can re-enter `withdraw()` before the
> balance is deducted, draining the vault. This is how the 2016 DAO
> hack worked — $60M lost.

---

### Fix 2 — Access Control

Add an `onlyOwner` modifier:
```solidity
modifier onlyOwner() {
    require(msg.sender == owner, "Not authorised");
    _;
}

function emergencyDrain() external onlyOwner { // ✅ restricted
    payable(msg.sender).transfer(address(this).balance);
}
```

> **Why?** Without access control, any address can drain the vault
> instantly. Always restrict privileged functions.

---

### Fix 3 — Remove Timestamp Dependence

Replace `block.timestamp` logic with an explicit admin-controlled window:
```solidity
bool public withdrawalOpen;

function setWithdrawalWindow(bool open) external onlyOwner {
    withdrawalOpen = open;
}

function isWithdrawalWindow() public view returns (bool) {
    return withdrawalOpen; // ✅ no miner manipulation possible
}
```

> **Why?** Miners can adjust `block.timestamp` by up to ~15 seconds,
> making time-based conditions exploitable in certain scenarios.

---

## Step 4 — Verify with Sanctifier

Run the analyzer again against the fixed contract:
```bash
sanctifier analyze contracts/VulnerableVault.sol
```

Expected output:
```
✅ No issues found. All checks passed.
```

---

## Step 5 — The Fixed Contract

Here is the complete fixed version for reference:
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title SecureVault
/// @notice Fixed version of VulnerableVault — tutorial reference
contract SecureVault {
    mapping(address => uint256) public balances;
    address public owner;
    bool public withdrawalOpen;

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Not authorised");
        _;
    }

    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }

    function withdraw(uint256 amount) external {
        require(isWithdrawalWindow(), "Withdrawal window closed");
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success, "Transfer failed");
    }

    function emergencyDrain() external onlyOwner {
        payable(msg.sender).transfer(address(this).balance);
    }

    function setWithdrawalWindow(bool open) external onlyOwner {
        withdrawalOpen = open;
    }

    function isWithdrawalWindow() public view returns (bool) {
        return withdrawalOpen;
    }
}
```

---

## What You Learned

| Vulnerability | Pattern | Fix |
|---|---|---|
| Reentrancy | External call before state update | Checks-Effects-Interactions |
| Missing access control | No modifier on privileged fn | `onlyOwner` modifier |
| Timestamp dependence | `block.timestamp` in logic | Admin-controlled state flag |

---

## Next Steps

- Run Sanctifier on your own contracts: `sanctifier analyze <path>`
- Explore the [full rule reference](./rules.md)
- Join the discussion on [GitHub](https://github.com/HyperSafeD/Sanctifier)