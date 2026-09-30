# Rollback Procedure — Mainnet Contract Deployment

**Covers realistic failure modes during or after a mainnet Soroban contract deployment and the concrete mitigation paths available, separate from the general deployment runbook.**

---

## 1. Incident Declaration & Authority

| Role | Authority | Contact |
|------|-----------|---------|
| **On-call engineer** | Declare incident, invoke circuit breaker (`pause`) | PagerDuty / Slack |
| **Lead maintainer** | Authorise proxy redeploy or timelocked upgrade | GitHub issue `/cc` |
| **Project admin (multisig)** | Emergency key rotation, on-chain admin actions | Defined in key-ceremony doc |

**Escalation path:** On-call → Lead maintainer → Project admin multisig.

A declared incident is tracked via a GitHub issue tagged `incident` and `P0`, with a post-mortem required within 5 business days.

---

## 2. Failure Scenarios & Mitigations

### Scenario A — Bad WASM Upload

**What happens:** Deploy transaction succeeds but the uploaded WASM is the wrong bytecode (wrong hash, stale build, or a non-production artifact).

**Detection:**
- WASM hash printed in deploy log differs from the CI-signed hash
- `soroban contract invoke --id <CID> -- version` returns unexpected value
- On-chain `IMPL_HASH` does not match the reference hash in the release manifest

**Mitigation path:**

1. **Pause via circuit breaker** (#1126)
   - Invoke `runtime-guard-wrapper.pause()` with the multisig admin key
   - This halts all guarded executions; state is frozen but readable
   - **Expected duration:** ~2 minutes

2. **Redeploy corrected WASM behind the same proxy** (contracts/proxy)
   - Build the correct WASM, obtain its hash from CI
   - Invoke `UupsProxy.upgrade(new_wasm_hash)` with the admin key
   - Verify new `IMPL_HASH` matches the expected release hash

3. **Verify & unpause**
   - Run `health_check` on the redeployed contract
   - Invoke `runtime-guard-wrapper.unpause()`
   - Confirm normal operation via `get_stats`

**If proxy is not available:** Deploy a fresh contract with corrected WASM, update all downstream consumers to point at the new contract ID.

### Scenario B — Failed / Partial Initialisation

**What happens:** The `initialize` call (or constructor-equivalent) fails mid-transaction, or succeeds but leaves storage in an inconsistent state. Soroban contract init is atomic, but if the init logic writes to multiple storage keys and one write fails silently (e.g. storage limit), the contract may be in a partially-configured state.

**Detection:**
- `health_check` returns `false` immediately after deploy
- `get_stats` returns zero invariants checked despite calls
- `get_wrapped_contract` returns an unexpected address or fails

**Mitigation path:**

1. **Freeze interactions** — the runtime guard (#1126) should already reject calls if `INITIALISED` is not set. Verify:
   - Invoke `runtime-guard-wrapper.pause()` if not already paused
   - All external calls return "Contract not initialised" error

2. **Diagnose** — inspect persistent storage keys via `soroban contract read`:
   - Check `WRAPPED_CONTRACT_ADDRESS`, `guard_config`, `CALL_LOG`
   - Compare against expected initial state from the deployment manifest

3. **Redeploy corrected contract** — if the proxy pattern is in use:
   - Deploy a corrected implementation WASM (with idempotent or fixed `initialize`)
   - Invoke `UupsProxy.upgrade(new_wasm_hash)` 
   - Invoke the corrected `initialize` path

4. **If proxy is not in use** — the broken contract is irrecoverable. Deploy an entirely new contract, update all registries/downstream consumers, and deprecate the old contract ID via an on-chain announcement.

### Scenario C — Post-Deploy Vulnerability Discovered

**What happens:** Hours or days after a successful deployment, a security vulnerability is reported (internally or via bug bounty) that affects the live contract.

**Detection:**
- Bug report filed via SECURITY.md disclosure path
- Internal audit or fuzzing run discovers a finding applicable to the deployed WASM
- On-chain events show anomalous guard failures (`guard_failure` events spiking)

**Mitigation path:**

1. **Emergency pause** — invoke `runtime-guard-wrapper.pause()` immediately
   - **Authority:** On-call engineer can invoke pause without further approval
   - This freezes all guarded state transitions

2. **Assess severity & impact**
   - Determine if funds/protected state are at immediate risk
   - If yes: keep paused, proceed to step 3
   - If no: consider timelocked upgrade path (#1127) for a more measured fix

3. **Deploy fix via timelock** (#1127) — the timelocked upgrade path requires a minimum delay (e.g. 24-48 hours) between proposal and execution:
   - Push a patched implementation WASM
   - Submit upgrade proposal to the `Timelock` contract
   - Wait for the delay period (allows watchers to review)
   - Execute the upgrade

4. **Emergency bypass** — if the vulnerability is actively exploited and timelock delay is unacceptably risky:
   - **Authority required:** Lead maintainer + 1 additional admin (2-of-3 multisig)
   - Invoke `UupsProxy.upgrade()` directly (bypassing timelock)
   - Document the emergency bypass in an incident report within 24 hours

5. **Unpause** after fix is verified, resume normal operations

### Scenario D — Storage Exhaustion / Fee Spike

**What happens:** The contract's storage grows beyond the Soroban per-contract storage limit, or the transaction fee spikes unexpectedly due to mainnet congestion, causing writes to fail mid-operation.

**Detection:**
- `soroban contract invoke` returns a storage limit error
- Deployment log shows fee-estimation warnings
- `health_check` fails due to `HEALTHY_STORAGE_LIMIT` being exceeded

**Mitigation path:**

1. **Pause contract** to prevent further storage writes
2. **Analyze storage usage** via `soroban contract read` and `get_stats`
3. **Prune stale data** if the contract supports data cleanup functions
4. **Redeploy with increased limits** or storage-optimised logic via proxy upgrade
5. **Unpause** after verification

### Scenario E — Admin Key Compromise

**What happens:** The admin key (deployer or upgrade authority) is leaked or compromised.

**Detection:**
- Unexpected `upgrade` or `transfer_admin` events in the event log
- Unauthorised `pause` / `unpause` transitions
- GitHub secret `SOROBAN_MAINNET_SECRET_KEY` is found in logs or external sources

**Mitigation path:**

1. **Rotate the admin key** — generate a new key pair via Soroban key generation
2. **Transfer admin** on all deployed contracts to the new key:
   - `UupsProxy.transfer_admin(new_admin_address)`
   - `new_admin_address` invokes `UupsProxy.accept_admin()`
3. **Revoke old key** — remove the compromised secret from GitHub Secrets
4. **Audit** event history for any unauthorised transactions made with the compromised key
5. **Incident report** with timeline and remediation steps

---

## 3. Post-Rollback Verification Checklist

After any rollback mitigation is applied:

- [ ] `health_check` returns `true` for all affected contracts
- [ ] `get_stats` shows expected invariants-checked count
- [ ] Guard events emit normally (test with a known-good transaction)
- [ ] Deployment manifest is updated with new contract IDs / WASM hashes
- [ ] GitHub Secrets are rotated if keys were exposed
- [ ] Incident post-mortem is drafted with root cause and preventive measures

---

## 4. Related Documents

- [Mainnet Deployment Runbook](./MAINNET_DEPLOYMENT_RUNBOOK.md) (#1134) — happy-path deployment
- `contracts/proxy/` — UUPS proxy pattern for WASM upgrades
- `contracts/runtime-guard-wrapper/` — on-chain circuit breaker (#1126)
- `contracts/timelock/` — timelocked upgrade governance (#1127)
- [RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md) — pre-release verification
- [BRANCH_PROTECTION.md](./BRANCH_PROTECTION.md) — branch and key protection policy
