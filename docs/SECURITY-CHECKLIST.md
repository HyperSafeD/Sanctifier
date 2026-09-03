# Soroban Smart Contract Security Self-Assessment Checklist

This checklist is designed to help Soroban contract developers self-assess their code before submitting it for a formal audit. 

## 1. Access Control & Authorization
- [ ] Are sensitive functions protected against unauthorized access? *(Sanctifier: S001 - Auth Gap)*
- [ ] Is `env.storage().instance().has()` used properly to check for initialization? *(Sanctifier: S008)*
- [ ] Are administrative roles well-defined and least-privileged? *(Manual Review)*
- [ ] Is `Address::require_auth()` called on the correct invoker for critical actions? *(Sanctifier: S001)*
- [ ] Is `Address::require_auth_for_args()` used when argument validation is required? *(Sanctifier: S001)*
- [ ] Are contract-to-contract authorization boundaries correctly handled? *(Sanctifier: S006)*
- [ ] Does the contract prevent arbitrary address impersonation? *(Sanctifier: S001)*

## 2. Arithmetic & Math Operations
- [ ] Are all mathematical operations safe from integer overflows and underflows? *(Sanctifier: S002)*
- [ ] Are divisions performed after multiplications to avoid precision loss? *(Sanctifier: S002)*
- [ ] Are checked math operations (`checked_add`, `checked_mul`) utilized where applicable? *(Sanctifier: S002)*
- [ ] Is zero-division explicitly checked and handled? *(Sanctifier: S002)*
- [ ] Are rounding directions correctly chosen for financial math (round in favor of protocol)? *(Manual Review)*

## 3. State & Data Validation
- [ ] Are all inputs from users validated against expected bounds and types? *(Sanctifier: S004)*
- [ ] Is `env.storage().persistent()` used for data that must outlive the contract? *(Sanctifier: S009)*
- [ ] Are keys for storage well-structured to avoid collisions? *(Sanctifier: S004)*
- [ ] Does the contract validate the length of input vectors/arrays? *(Sanctifier: S004)*
- [ ] Are time-based conditions (e.g., locks, expirations) securely validating the ledger time? *(Sanctifier: S009)*
- [ ] Is the data read from external contracts validated before use? *(Manual Review)*

## 4. Cryptography & Randomness
- [ ] Does the contract avoid using ledger sequence as a source of secure randomness? *(Sanctifier: S003)*
- [ ] Are cryptographic primitives implemented securely using Soroban's built-in functions? *(Sanctifier: S003)*
- [ ] Is signature verification done through `env.crypto().ed25519_verify()` correctly? *(Sanctifier: S003)*
- [ ] Are nonces used to prevent replay attacks if custom signature schemes are built? *(Manual Review)*

## 5. Denial of Service (DoS) & Resource Limits
- [ ] Are iteration loops bounded to prevent exceeding CPU limits? *(Sanctifier: S005)*
- [ ] Is memory allocation bounded for input data types to avoid memory exhaustion? *(Sanctifier: S005)*
- [ ] Are large structures avoided in local scope/stack? *(Sanctifier: S005)*
- [ ] Does the contract prevent griefing attacks where malicious users lock shared resources? *(Manual Review)*

## 6. Cross-Contract Calls & Reentrancy
- [ ] Is reentrancy mitigated natively or logically (Soroban handles some, but logical reentrancy exists)? *(Sanctifier: S006)*
- [ ] Are state changes written to storage before calling external contracts (Checks-Effects-Interactions)? *(Sanctifier: S006)*
- [ ] Are errors from cross-contract calls handled securely without panicking the parent unless intended? *(Sanctifier: S006)*
- [ ] Are external contract IDs validated before invoking them? *(Manual Review)*

## 7. Business Logic & Edge Cases
- [ ] Are all token transfers verifying balance before transferring? *(Sanctifier: S007)*
- [ ] Are edge cases like 0-value transfers explicitly handled? *(Sanctifier: S007)*
- [ ] Is fee calculation logic correct and immune to manipulation? *(Manual Review)*
- [ ] Can the contract handle a paused or deprecated state if implemented? *(Manual Review)*
- [ ] Are staking/reward logic distributions tested for edge cases (e.g., 0 stakers)? *(Manual Review)*
- [ ] Is order-of-execution dependence minimized in Defi constructs? *(Manual Review)*

## 8. Initialization & Upgradability
- [ ] Is the initialization function protected against being called twice? *(Sanctifier: S008)*
- [ ] Is the upgrade mechanism protected by a secure multisig or DAO? *(Sanctifier: S008)*
- [ ] Does the upgraded contract validate the existing storage schema? *(Manual Review)*
- [ ] Is `env.deployer()` used securely if deploying child contracts? *(Sanctifier: S008)*
- [ ] Is the WASM hash verified before upgrading? *(Manual Review)*

## 9. Environment & Soroban APIs
- [ ] Are deprecated Soroban environment functions avoided? *(Sanctifier: S009)*
- [ ] Is the correct storage type (`persistent`, `temporary`, `instance`) chosen for the data lifecycle? *(Sanctifier: S009)*
- [ ] Are `require_auth` boundaries spanning across contract calls managed safely? *(Sanctifier: S009)*
- [ ] Is ledger sequence `env.ledger().sequence()` used appropriately without expecting exact time synchronization? *(Sanctifier: S009)*

## 10. Error Handling & Panics
- [ ] Does the contract use explicit `Error` enums instead of native `panic!`? *(Sanctifier: S010)*
- [ ] Are internal logic panics avoided during user operations? *(Sanctifier: S010)*
- [ ] Do errors expose sensitive internal state information? *(Sanctifier: S010)*
- [ ] Are `unwrap()` and `expect()` calls justified and safe from crashing on user input? *(Sanctifier: S010)*

## 11. Event Emission
- [ ] Are all critical state changes emitting events? *(Sanctifier: S011)*
- [ ] Do events avoid logging sensitive data or private keys? *(Sanctifier: S011)*
- [ ] Are event topics properly structured for efficient indexing? *(Manual Review)*

## 12. Rust/Wasm Specifics
- [ ] Is the WASM binary optimized for size to meet network constraints? *(Manual Review)*
- [ ] Does the code avoid unsafe Rust blocks? *(Sanctifier: S012)*
- [ ] Are all Rust warnings and clippy lints addressed? *(Sanctifier: S012)*
- [ ] Is the `no_std` environment strictly adhered to? *(Sanctifier: S012)*
