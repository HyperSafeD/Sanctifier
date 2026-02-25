# Runtime Guard Wrapper Contract

## Overview

The Runtime Guard Wrapper is a Soroban smart contract designed to wrap other contracts and provide real-time validation, monitoring, and security guards during execution.

## Purpose

This contract enables:

1. **Pre-execution Validation** - Verify system state before function execution
2. **Post-execution Verification** - Confirm state invariants after execution
3. **Execution Monitoring** - Track and record all contract calls
4. **Security Guards** - Enforce runtime contracts and policies
5. **Metrics Collection** - Gather performance and operational data
6. **Event Emission** - Emit events for external monitoring systems
7. **Continuous Validation** - Enable testnet continuous validation requirements

## Architecture

### System Components

```
┌──────────────────────────────────────────────────────────────┐
│                    Soroban Testnet                           │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │        Runtime Guard Wrapper Contract                  │ │
│  │  ┌──────────┐                                          │ │
│  │  │  A. Pre-Execution Guards                           │ │
│  │  │  - Verify wrapped contract set                     │ │
│  │  │  - Check storage integrity                         │ │
│  │  └──────────┘                                          │ │
│  │         ↓                                               │ │
│  │  ┌──────────┐                                          │ │
│  │  │  B. Execute with Monitoring                        │ │
│  │  │  - Record metrics                                  │ │
│  │  │  - Track gas usage                                 │ │
│  │  │  - Measure execution time                          │ │
│  │  └──────────┘                                          │ │
│  │         ↓                                               │ │
│  │  ┌──────────┐                                          │ │
│  │  │  C. Post-Execution Guards                          │ │
│  │  │  - Verify invariants                               │ │
│  │  │  - Check state consistency                         │ │
│  │  │  - Emit validation events                          │ │
│  │  └──────────┘                                          │ │
│  │         ↓                                               │ │
│  │  ┌──────────┐                                          │ │
│  │  │  D. Logging & Reporting                            │ │
│  │  │  - Record execution                                │ │
│  │  │  - Collect statistics                              │ │
│  │  │  - Update metrics                                  │ │
│  │  └──────────┘                                          │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Key Features

### 1. Guard Configuration

```rust
pub struct GuardConfig {
    pub check_storage_invariants: bool,    // Validate storage state
    pub check_auth_guards: bool,            // Verify authorization
    pub check_overflow: bool,               // Detect overflows
    pub monitor_events: bool,               // Track events
    pub max_execution_time_ms: u32,         // Execution timeout
}
```

### 2. Execution Flow

```
Initialize Wrapper
    ↓
Set Wrapped Contract Address
    ↓
Pre-Execution Checks
    ├─ Wrapped contract accessible?
    ├─ Storage integrity OK?
    └─ Guards configured?
    ↓
Execute Guarded Function
    ├─ Record start time
    ├─ Monitor execution
    ├─ Handle errors
    └─ Record metrics
    ↓
Post-Execution Checks
    ├─ Verify invariants
    ├─ Check consistency
    └─ Emit events
    ↓
Log & Report
    ├─ Update call log
    ├─ Record metrics
    └─ Update statistics
```

### 3. Storage Management

The contract uses three storage layers:

**Instance Storage** (Persistent):
- Wrapped contract address
- Guard configuration

**Persistent Storage** (Long-term):
- Call execution log (capped at 100 entries)
- Invariant check counter
- Guard failure records
- Execution metrics (capped at 1000 entries)
- Event logs

## Public Interface

### `init(env: Env, wrapped_contract: Address)`

Initialize the wrapper with a target contract address.

```rust
pub fn init(env: Env, wrapped_contract: Address) {
    // Set wrapped contract
    // Initialize guard configuration
    // Initialize logging
    // Emit initialization event
}
```

### `get_wrapped_contract(env: Env) -> Address`

Retrieve the wrapped contract address.

```ruby
pub fn get_wrapped_contract(env: Env) -> Address {
    // Returns the address of the wrapped contract
}
```

### `execute_guarded(env: Env, function_name: Symbol, args: Vec<Val>) -> Result<Val, Error>`

Execute a function with runtime guards enabled.

```ruby
pub fn execute_guarded(
    env: Env,
    function_name: Symbol,
    args: Vec<Val>
) -> Result<Val, Error> {
    // Pre-execution guards
    // Monitored execution
    // Post-execution checks
    // Logging
    // Returns result or error
}
```

### `get_stats(env: Env) -> (u32, u32, u32)`

Get contract statistics.

Returns tuple: `(invariants_checked, execution_count, guard_failures)`

```rust
pub fn get_stats(env: Env) -> (u32, u32, u32) {
    // Returns statistics for monitoring
}
```

### `health_check(env: Env) -> bool`

Verify contract health and operational status.

```rust
pub fn health_check(env: Env) -> bool {
    // Checks:
    // - Wrapped contract is set
    // - Metrics storage accessible
    // Returns true if all systems operational
}
```

## Execution Metrics

Each execution generates metrics:

```rust
pub struct ExecutionMetrics {
    pub call_hash: u32,        // Hash of function call
    pub success: bool,          // Execution succeeded?
    pub timestamp: u64,         // Execution time
    pub gas_used: u64,          // Gas consumed (reserved)
}
```

## Event Emission

The contract emits events for monitoring:

```
Event: (Symbol, Symbol)
First: Event name (e.g., "guard_wrapper", "pre_exec_guard")
Second: Status (e.g., "success", "passed", "failure")
```

## Integration Guide

### 1. Build the Contract

```bash
cargo build \
  -p runtime-guard-wrapper \
  --release \
  --target wasm32-unknown-unknown
```

### 2. Deploy to Testnet

```bash
# Using deployment script
bash scripts/deploy-soroban-testnet.sh --network testnet

# Or using CLI
sanctifier deploy ./contracts/runtime-guard-wrapper \
  --network testnet \
  --validate
```

### 3. Initialize Wrapper

```bash
CONTRACT_ID="C1234567..."
WRAPPED_CONTRACT="GXXXXX..."

soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network testnet \
  -- init "$WRAPPED_CONTRACT"
```

### 4. Execute Guarded Functions

```bash
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network testnet \
  -- execute_guarded function_name args
```

### 5. Monitor Operations

```bash
# Get statistics
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network testnet \
  -- get_stats

# Health check
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network testnet \
  -- health_check

# Read events
soroban events read \
  --network testnet \
  --id "$CONTRACT_ID"
```

## Testing

The contract includes integration tests:

```bash
cd contracts/runtime-guard-wrapper
cargo test
```

Test coverage:
- Wrapper initialization
- Pre-execution guards
- Post-execution guards
- Execution logging
- Metrics recording
- Health checks
- Event emission
- Storage validation

## Security Considerations

### Design Principles

1. **Guard Immutability** - Guards cannot be disabled after initialization
2. **Storage Integrity** - Regular validation of critical storage
3. **Audit Trail** - All operations are logged
4. **Fail-Safe** - Errors prevent execution
5. **Bounded Growth** - Logs and metrics are capped to prevent DoS

### Guard Coverage

- ✅ Pre-execution: Verify system readiness
- ✅ Post-execution: Confirm state consistency
- ✅ Overflow: Detect arithmetic errors
- ⚠️ Authorization: Guard invocation restrictions
- ⚠️ Events: Monitor critical operations

### Limitations

- Wrapped contract address cannot be changed after init
- Storage space is bounded (100 calls, 1000 metrics)
- Events are advisory only (not enforceable)
- No on-chain cryptographic verification

## Performance

### Gas Costs

Estimated gas usage per operation:

| Operation | Approximate Gas |
|-----------|-----------------|
| init | 15,000 |
| execute_guarded | 10,000-20,000 |
| get_stats | 5,000 |
| health_check | 8,000 |

### Storage Usage

| Item | Size |
|------|------|
| Wrapped address | 32 bytes |
| Config | 16 bytes |
| Call log entry | 32 bytes (×100 max) |
| Metrics entry | 32 bytes (×1000 max) |
| **Total max** | ~64 KB |

## Future Enhancements

Potential improvements:
- [ ] Async invocation support
- [ ] Custom guard policies
- [ ] Per-function rate limiting
- [ ] Advanced metrics (percentiles, histograms)
- [ ] Replay protection
- [ ] Multi-wrapper composition
- [ ] Event filtering and sampling

## Troubleshooting

### "Wrapped contract not found"
```bash
# Verify contract is initialized
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network testnet \
  -- get_wrapped_contract
```

### "Storage integrity check failed"
```bash
# Run health check
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network testnet \
  -- health_check

# Check contract state
soroban contract read --id "$CONTRACT_ID" --network testnet
```

### "Execution timeout"
- May indicate network congestion
- Increase timeout in guard config
- Check RPC endpoint health

## References

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Contract Interface](./src/lib.rs)
- [Integration Tests](./tests/integration_tests.rs)
- [Deployment Guide](../docs/soroban-deployment.md)

## License

Part of the Sanctifier project. See repository LICENSE.
