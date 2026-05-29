# Sanctifier Live-Testnet Walkthrough — Video Script

> **Format:** Markdown storyboard with timing cues and voiceover-friendly prose.  
> **Target runtime:** ~8 minutes.  
> **Audience:** Soroban developers new to Sanctifier.

---

## [00:00 – 00:30] Intro

**[SCREEN: Sanctifier GitHub README hero image]**

> "Welcome to Sanctifier — a runtime security framework for Soroban smart contracts on Stellar.
> In this walkthrough I'll show you how to interact with a live deployment on Soroban Testnet,
> verify the on-chain guard, and trigger a real audit event — all in under ten minutes."

---

## [00:30 – 01:30] What We're Working With

**[SCREEN: LIVE_TESTNET.md — Deployed Contract Addresses table]**

> "Sanctifier ships three contracts on testnet.
> The **Runtime Guard Wrapper** is the brain — it intercepts every call and emits audit events.
> The **Vulnerable Contract** is a demo target we'll poke at.
> The **Reentrancy Guard** shows how a guard plugin integrates.
>
> All three addresses are live right now. You can follow along on
> Stellar Expert while I run these commands."

---

## [01:30 – 02:30] Prerequisites

**[SCREEN: terminal — blank prompt]**

> "You need two things: the Stellar CLI and a funded testnet keypair.
> If you haven't done that yet, pause here and run:"

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Generate a keypair and fund it from Friendbot
stellar keys generate --global demo --network testnet
stellar keys fund demo --network testnet
```

> "Once your balance shows up on the explorer, come back and we'll start querying."

---

## [02:30 – 03:30] Health Check

**[SCREEN: terminal]**

> "First, let's confirm the Runtime Guard Wrapper is alive."

```bash
stellar contract invoke \
  --id CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB \
  --source demo \
  --network testnet \
  -- health_check
```

> "You should see `true` come back almost instantly.
> If it times out, the RPC endpoint may be congested — wait 30 seconds and retry."

**[SCREEN: terminal output showing `true`]**

---

## [03:30 – 05:00] Reading Aggregate Statistics

**[SCREEN: terminal]**

> "Now let's pull the live audit counters — how many calls the guard has wrapped
> and how many violations it has caught so far."

```bash
stellar contract invoke \
  --id CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB \
  --source demo \
  --network testnet \
  -- get_stats
```

> "The response is a JSON object with `total_calls`, `violations`, and `last_block`.
> Every time someone invokes the vulnerable contract through the wrapper, `total_calls` ticks up.
> A reentrancy attempt would bump `violations`."

**[SCREEN: JSON output highlighted]**

---

## [05:00 – 06:30] Triggering an Audit Event

**[SCREEN: terminal]**

> "Let's make a call through the guard so we can watch an event land on-chain."

```bash
stellar contract invoke \
  --id CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB \
  --source demo \
  --network testnet \
  -- wrapped_call \
  --target CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB \
  --payload '"ping"'
```

> "This routes through the guard wrapper into the vulnerable contract.
> Switch over to Stellar Expert and open the Runtime Guard Wrapper contract.
> Under **Events** you'll see a new `audit_call` event appear within a few seconds."

**[SCREEN: split — terminal left, Stellar Expert events tab right]**

---

## [06:30 – 07:30] Verifying On-chain Events in Stellar Expert

**[SCREEN: browser — Stellar Expert contract page]**

> "Click on the latest `audit_call` event.
> You'll see the caller address, the target contract, the payload hash,
> and a `safe: true` flag — meaning no policy rule was violated.
>
> If you try to trigger a reentrancy — calling back into the guard mid-execution —
> that flag flips to `safe: false` and the transaction reverts.
> That's Sanctifier doing its job."

---

## [07:30 – 08:00] Outro

**[SCREEN: Sanctifier README]**

> "That's the full live-testnet loop: health check, read stats, trigger a call, verify the event.
>
> To go deeper — integrating your own contracts with the guard, writing custom policy rules,
> or running the full test suite — check the links in the description.
>
> Star the repo if this was helpful, and open an issue if you hit anything unexpected.
> See you in the next one."

---

## Reference Links

| Resource | URL |
|---|---|
| Stellar Expert — Runtime Guard | https://stellar.expert/explorer/testnet/contract/CBLDEREKXK6AIZ7ZSKC6VYCK4MKF4FZ4ANJEU67QZAQUG57I4KGZMTXB |
| Stellar Expert — Vulnerable Contract | https://stellar.expert/explorer/testnet/contract/CABBT5FKG7AE7IEEA4KR2J5AVYRSZAWKTXZ2KFX3UNJQAMMLMCXNLMIB |
| Stellar Expert — Reentrancy Guard | https://stellar.expert/explorer/testnet/contract/CDDVM5A5IVDAG5FZ2OU2CLWAHC7A2T7LHQHZSDVKZPE6SDMDO2JCR3UY |
| Soroban Testnet RPC | https://soroban-testnet.stellar.org |
| Stellar CLI Docs | https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli |
