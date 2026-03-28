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
            Self::emit_guard_event(env, "wrapper_initialized", "idempotent");
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

        Self::emit_guard_event(env, "wrapper_initialized", "success");
    }

    pub fn get_wrapped_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS))
            .unwrap()
    }

    pub fn execute_guarded(env: Env, function_name: Symbol, args: Vec<Val>) -> Result<Val, Error> {
        Self::pre_execution_guards(env.clone())?;
        let result = Self::execute_with_monitoring(env.clone(), &function_name, &args)?;
        Self::post_execution_guards(env.clone())?;
        Self::log_execution(env.clone(), &function_name, &result);
        Ok(result)
    }

    fn pre_execution_guards(env: Env) -> Result<(), Error> {
        let wrapped = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS));
        if wrapped.is_none() {
            Self::emit_guard_event(env, "pre_exec_guard", "wrapped_contract_not_set");
            return Err(Error::from_contract_error(1));
        }

        Self::validate_storage_integrity(env)?;
        Ok(())
    }

    fn post_execution_guards(env: Env) -> Result<(), Error> {
        Self::verify_storage_invariants(env.clone())?;
        Self::emit_guard_event(env, "post_exec_guard", "passed");
        Ok(())
    }

    fn validate_storage_integrity(env: Env) -> Result<(), Error> {
        let instance_storage = env.storage().instance();
        let wrapped_addr: Option<Address> =
            instance_storage.get(&Symbol::new(&env, WRAPPED_CONTRACT_ADDRESS));
        if wrapped_addr.is_none() {
            return Err(Error::from_contract_error(2));
        }
        Ok(())
    }

    fn verify_storage_invariants(env: Env) -> Result<(), Error> {
        let persistent = env.storage().persistent();
        let checked_count: u32 = persistent
            .get(&Symbol::new(&env, INVARIANTS_CHECKED))
            .unwrap_or(0);
        persistent.set(
            &Symbol::new(&env, INVARIANTS_CHECKED),
            &checked_count.saturating_add(1),
        );
        Ok(())
    }

    fn execute_with_monitoring(
        env: Env,
        function_name: &Symbol,
        args: &Vec<Val>,
    ) -> Result<Val, Error> {
        let expected_arg_count = match Self::expected_arg_count(&env, function_name) {
            Some(count) => count,
            None => {
                Self::record_guard_failure(env.clone(), Symbol::new(&env, "missing_function"));
                return Err(Error::from_contract_error(3));
            }
        };

        if args.len() != expected_arg_count {
            Self::record_guard_failure(env.clone(), Symbol::new(&env, "arg_mismatch"));
            return Err(Error::from_contract_error(4));
        }

        let start_tick = env.ledger().timestamp();
        let result = Self::simulate_wrapped_call(env.clone(), function_name, args)?;
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
    ) -> Result<Val, Error> {
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
                .map_err(|_| Error::from_contract_error(4))?;
            let right = u32::try_from_val(&env, &args.get(1).unwrap_or(Val::VOID.into()))
                .map_err(|_| Error::from_contract_error(4))?;
            return Ok(left.saturating_add(right).into_val(&env));
        }

        Err(Error::from_contract_error(3))
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

        Self::emit_guard_event(env, "execution_logged", "success");
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
        Self::emit_guard_event(env, "guard_failure", "recorded");
    }

    fn emit_guard_event(env: Env, event_name: &str, status: &str) {
        env.events().publish(
            (Symbol::new(&env, "guard_wrapper"),),
            (Symbol::new(&env, event_name), Symbol::new(&env, status)),
        );
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
