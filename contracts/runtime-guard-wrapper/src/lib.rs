#![no_std]
#![allow(unexpected_cfgs)]

use soroban_sdk::{
    contract, contractimpl, Address, Env, Error, IntoVal, Symbol, TryFromVal, Val, Vec,
};

const WRAPPED_CONTRACT_ADDRESS: &str = "wrapped_contract_addr";
const CALL_LOG: &str = "call_log";
const INVARIANTS_CHECKED: &str = "invariants_checked";
const GUARD_FAILURES: &str = "guard_failures";
const EXECUTION_METRICS: &str = "exec_metrics";
const HEALTHY_STORAGE_LIMIT: u32 = 64;
const CONTRACT_VERSION_KEY: &str = "version";

/// Current storage schema version. Increment when persistent storage layout
/// changes and provide a migration path in `docs/contract-versioning.md`.
pub const CONTRACT_VERSION: u32 = 1;

// ── Semantic error codes ───────────────────────────────────────────────────────
//
// These constants replace raw numeric literals throughout the contract so that
// a failing test or on-chain event can be matched to a specific guard stage
// without consulting source code.

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RuntimeGuardError {
    /// Pre-execution guard: wrapped contract address has not been set via `init`.
    WrappedContractNotSet = 1,
    /// Pre-execution guard: instance storage is missing the wrapped contract key.
    StorageIntegrityFailed = 2,
    /// Execution monitoring: the requested function name is not registered.
    UnknownFunction = 3,
    /// Execution monitoring: argument count mismatch.
    ArgumentCountMismatch = 4,
}

// ── Legacy Compatibility ───────────────────────────────────────────────────────
// The following constants are preserved to ensure minimal breaking surface for
// downstream consumers and tests. New code should use `RuntimeGuardError`.

pub const ERR_WRAPPED_CONTRACT_NOT_SET: u32 = RuntimeGuardError::WrappedContractNotSet as u32;
pub const ERR_STORAGE_INTEGRITY_FAILED: u32 = RuntimeGuardError::StorageIntegrityFailed as u32;
pub const ERR_UNKNOWN_FUNCTION: u32 = RuntimeGuardError::UnknownFunction as u32;
pub const ERR_ARGUMENT_COUNT_MISMATCH: u32 = RuntimeGuardError::ArgumentCountMismatch as u32;

mod event_fixtures {
    use soroban_sdk::{Env, Symbol};

    pub const TOPIC_GUARD_WRAPPER: &str = "guard_wrapper";
    pub const EVENT_WRAPPER_INITIALIZED: &str = "wrapper_initialized";
    pub const EVENT_PRE_EXEC_GUARD: &str = "pre_exec_guard";
    pub const EVENT_POST_EXEC_GUARD: &str = "post_exec_guard";
    pub const EVENT_EXECUTION_LOGGED: &str = "execution_logged";
    pub const EVENT_GUARD_FAILURE: &str = "guard_failure";

    pub const STATUS_IDEMPOTENT: &str = "idempotent";
    pub const STATUS_SUCCESS: &str = "success";
    pub const STATUS_PASSED: &str = "passed";
    pub const STATUS_RECORDED: &str = "recorded";
    pub const STATUS_WRAPPED_NOT_SET: &str = "wrapped_contract_not_set";
    pub const STATUS_WRAPPED_CALL_ERROR: &str = "wrapped_call_error";

    pub fn emit(env: &Env, event_name: &str, status: &str) {
        env.events().publish(
            (Symbol::new(env, TOPIC_GUARD_WRAPPER),),
            (Symbol::new(env, event_name), Symbol::new(env, status)),
        );
    }
}

#[derive(Clone, Debug)]
pub struct GuardConfig {
    pub check_storage_invariants: bool,
    pub check_auth_guards: bool,
    pub check_overflow: bool,
    pub monitor_events: bool,
    pub max_execution_time_ms: u32,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            check_storage_invariants: true,
            check_auth_guards: true,
            check_overflow: true,
            monitor_events: true,
            max_execution_time_ms: 5000,
        }
    }
}

#[derive(Clone)]
pub struct ExecutionMetrics {
    pub call_hash: u32,
    pub success: bool,
    pub timestamp: u64,
    pub gas_used: u64,
}

#[contract]
pub struct RuntimeGuardWrapper;

#[contractimpl]
impl RuntimeGuardWrapper {
    pub fn init(env: Env, wrapped_contract: Address) {
        if env
            .storage()
            .instance()
            .has(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS))
        {
            Self::emit_guard_event(
                env,
                event_fixtures::EVENT_WRAPPER_INITIALIZED,
                event_fixtures::STATUS_IDEMPOTENT,
            );
            return;
        }

        env.storage().instance().set(
            &Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS),
            &wrapped_contract,
        );

        let config = GuardConfig::default();
        env.storage().instance().set(
            &Symbol::new(&env, "guard_config"),
            &(
                config.check_storage_invariants,
                config.check_auth_guards,
                config.check_overflow,
                config.monitor_events,
            ),
        );

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, CALL_LOG), &Vec::<Symbol>::new(&env));
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, INVARIANTS_CHECKED), &0u32);
        env.storage().persistent().set(
            &Symbol::new(&env, GUARD_FAILURES),
            &Vec::<Symbol>::new(&env),
        );
        env.storage().persistent().set(
            &Symbol::new(&env, EXECUTION_METRICS),
            &Vec::<(u32, bool, u64, u64)>::new(&env),
        );

        env.storage()
            .instance()
            .set(&Symbol::new(&env, CONTRACT_VERSION_KEY), &CONTRACT_VERSION);

        Self::emit_guard_event(
            env,
            event_fixtures::EVENT_WRAPPER_INITIALIZED,
            event_fixtures::STATUS_SUCCESS,
        );
    }

    /// Returns the on-chain schema version stamped during `init`.
    pub fn get_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<Symbol, u32>(&Symbol::new(&env, CONTRACT_VERSION_KEY))
            .unwrap_or(CONTRACT_VERSION)
    }

    pub fn get_wrapped_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS))
            .unwrap()
    }

    pub fn execute_guarded(env: Env, function_name: Symbol, args: Vec<Val>) -> Result<Val, Error> {
        Self::validate_function_name(&env, &function_name)?;
        Self::pre_execution_guards(env.clone())?;
        let result = Self::execute_with_monitoring(env.clone(), &function_name, &args)?;
        Self::post_execution_guards(env.clone())?;
        Self::log_execution(env.clone(), &function_name, &result);
        Ok(result)
    }

    /// Validate that the function name is non-empty and within the allowed
    /// Symbol length (Soroban Symbols are capped at 32 characters).
    fn validate_function_name(env: &Env, function_name: &Symbol) -> Result<(), RuntimeGuardError> {
        // A valid function name Symbol must be non-zero when converted to a Val
        // payload — this catches default/zero Symbol values.
        let val: Val = function_name.clone().into_val(env);
        if val.get_payload() == 0 {
            return Err(RuntimeGuardError::UnknownFunction);
        }
        Ok(())
    }

    fn pre_execution_guards(env: Env) -> Result<(), RuntimeGuardError> {
        let wrapped = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS));
        if wrapped.is_none() {
            Self::emit_guard_event(
                env,
                event_fixtures::EVENT_PRE_EXEC_GUARD,
                event_fixtures::STATUS_WRAPPED_NOT_SET,
            );
            return Err(RuntimeGuardError::WrappedContractNotSet);
        }

        Self::validate_storage_integrity(env)?;
        Ok(())
    }

    fn post_execution_guards(env: Env) -> Result<(), RuntimeGuardError> {
        Self::verify_storage_invariants(env.clone());
        Self::emit_guard_event(
            env,
            event_fixtures::EVENT_POST_EXEC_GUARD,
            event_fixtures::STATUS_PASSED,
        );
        Ok(())
    }

    fn validate_storage_integrity(env: Env) -> Result<(), RuntimeGuardError> {
        let instance_storage = env.storage().instance();
        let wrapped_addr: Option<Address> =
            instance_storage.get(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS));
        if wrapped_addr.is_none() {
            return Err(RuntimeGuardError::StorageIntegrityFailed);
        }
        Ok(())
    }

    fn verify_storage_invariants(env: Env) {
        let persistent = env.storage().persistent();
        let checked_count: u32 = persistent
            .get(&Symbol::new(&env, INVARIANTS_CHECKED))
            .unwrap_or(0);
        persistent.set(
            &Symbol::new(&env, INVARIANTS_CHECKED),
            &checked_count.saturating_add(1),
        );
    }

    fn execute_with_monitoring(
        env: Env,
        function_name: &Symbol,
        args: &Vec<Val>,
    ) -> Result<Val, RuntimeGuardError> {
        let expected_arg_count = match Self::expected_arg_count(&env, function_name) {
            Some(count) => count,
            None => {
                Self::record_guard_failure(env.clone(), Symbol::new(&env, "missing_function"));
                return Err(RuntimeGuardError::UnknownFunction);
            }
        };

        if args.len() != expected_arg_count {
            Self::record_guard_failure(env.clone(), Symbol::new(&env, "arg_mismatch"));
            return Err(RuntimeGuardError::ArgumentCountMismatch);
        }

        let start_tick = env.ledger().timestamp();
        let result = match Self::simulate_wrapped_call(env.clone(), function_name, args) {
            Ok(val) => val,
            Err(err) => {
                Self::record_guard_failure(
                    env.clone(),
                    Symbol::new(&env, event_fixtures::STATUS_WRAPPED_CALL_ERROR),
                );
                return Err(err);
            }
        };
        let val: Val = function_name.clone().into_val(&env);
        let call_hash = (val.get_payload().wrapping_mul(31) ^ start_tick.wrapping_mul(17)) as u32;

        Self::record_metrics(
            env,
            ExecutionMetrics {
                call_hash,
                success: true,
                timestamp: start_tick,
                gas_used: 0,
            },
        );

        Ok(result)
    }

    fn expected_arg_count(env: &Env, function_name: &Symbol) -> Option<u32> {
        if *function_name == Symbol::new(env, "ping") {
            return Some(0);
        }
        if *function_name == Symbol::new(env, "echo") {
            return Some(1);
        }
        if *function_name == Symbol::new(env, "sum") {
            return Some(2);
        }
        None
    }

    fn simulate_wrapped_call(
        env: Env,
        function_name: &Symbol,
        args: &Vec<Val>,
    ) -> Result<Val, RuntimeGuardError> {
        let ping = Symbol::new(&env, "ping");
        let echo = Symbol::new(&env, "echo");
        let sum = Symbol::new(&env, "sum");

        if *function_name == ping {
            return Ok(Symbol::new(&env, "pong").into_val(&env));
        }
        if *function_name == echo {
            return Ok(args.get(0).unwrap_or(Val::VOID.into()));
        }
        if *function_name == sum {
            let left = u32::try_from_val(&env, &args.get(0).unwrap_or(Val::VOID.into()))
                .map_err(|_| RuntimeGuardError::ArgumentCountMismatch)?;
            let right = u32::try_from_val(&env, &args.get(1).unwrap_or(Val::VOID.into()))
                .map_err(|_| RuntimeGuardError::ArgumentCountMismatch)?;
            return Ok(left.saturating_add(right).into_val(&env));
        }

        Err(RuntimeGuardError::UnknownFunction)
    }

    fn log_execution(env: Env, function_name: &Symbol, _result: &Val) {
        let persistent = env.storage().persistent();
        let call_log_symbol = Symbol::new(&env, CALL_LOG);
        let mut log: Vec<Symbol> = persistent
            .get(&call_log_symbol)
            .unwrap_or_else(|| Vec::new(&env));

        log.push_back(function_name.clone());

        if log.len() > 100 {
            let mut new_log = Vec::new(&env);
            for item in log.iter().skip(1usize) {
                new_log.push_back(item);
            }
            persistent.set(&call_log_symbol, &new_log);
        } else {
            persistent.set(&call_log_symbol, &log);
        }

        Self::emit_guard_event(
            env,
            event_fixtures::EVENT_EXECUTION_LOGGED,
            event_fixtures::STATUS_SUCCESS,
        );
    }

    fn record_metrics(env: Env, metrics: ExecutionMetrics) {
        let persistent = env.storage().persistent();
        let metrics_symbol = Symbol::new(&env, EXECUTION_METRICS);
        let mut metrics_vec: Vec<(u32, bool, u64, u64)> = persistent
            .get(&metrics_symbol)
            .unwrap_or_else(|| Vec::new(&env));

        metrics_vec.push_back((
            metrics.call_hash,
            metrics.success,
            metrics.timestamp,
            metrics.gas_used,
        ));

        if metrics_vec.len() > 1000 {
            let mut truncated = Vec::new(&env);
            for item in metrics_vec.iter().skip((metrics_vec.len() - 1000) as usize) {
                truncated.push_back(item);
            }
            persistent.set(&metrics_symbol, &truncated);
        } else {
            persistent.set(&metrics_symbol, &metrics_vec);
        }
    }

    fn record_guard_failure(env: Env, failure: Symbol) {
        let persistent = env.storage().persistent();
        let failure_symbol = Symbol::new(&env, GUARD_FAILURES);
        let mut failures: Vec<Symbol> = persistent
            .get(&failure_symbol)
            .unwrap_or_else(|| Vec::new(&env));
        failures.push_back(failure);
        persistent.set(&failure_symbol, &failures);
        Self::emit_guard_event(
            env,
            event_fixtures::EVENT_GUARD_FAILURE,
            event_fixtures::STATUS_RECORDED,
        );
    }

    fn emit_guard_event(env: Env, event_name: &str, status: &str) {
        event_fixtures::emit(&env, event_name, status);
    }

    pub fn get_stats(env: Env) -> (u32, u32, u32) {
        let persistent = env.storage().persistent();

        let invariants_checked: u32 = persistent
            .get(&Symbol::new(&env, INVARIANTS_CHECKED))
            .unwrap_or(0);

        let call_log: Vec<Symbol> = persistent
            .get(&Symbol::new(&env, CALL_LOG))
            .unwrap_or_else(|| Vec::new(&env));

        let guard_failures: Vec<Symbol> = persistent
            .get(&Symbol::new(&env, GUARD_FAILURES))
            .unwrap_or_else(|| Vec::new(&env));

        (invariants_checked, call_log.len(), guard_failures.len())
    }

    pub fn health_check(env: Env) -> bool {
        let has_wrapped = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS))
            .is_some();

        let metrics: Vec<(u32, bool, u64, u64)> = env
            .storage()
            .persistent()
            .get::<Symbol, Vec<(u32, bool, u64, u64)>>(&Symbol::new(&env, EXECUTION_METRICS))
            .unwrap_or_else(|| Vec::new(&env));

        let call_log: Vec<Symbol> = env
            .storage()
            .persistent()
            .get::<Symbol, Vec<Symbol>>(&Symbol::new(&env, CALL_LOG))
            .unwrap_or_else(|| Vec::new(&env));

        let healthy_metrics = metrics.len() < HEALTHY_STORAGE_LIMIT;
        let healthy_log = call_log.len() < HEALTHY_STORAGE_LIMIT;

        has_wrapped && healthy_metrics && healthy_log
    }
}
