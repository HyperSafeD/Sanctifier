use criterion::{criterion_group, criterion_main, Criterion};
use sanctifier_core::{Analyzer, SanctifyConfig};

const COMPLEX_CONTRACT_PAYLOAD: &str = r#"
#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, vec, Env, Symbol, Vec, Address};

#[contracttype]
pub struct ComplexStorage {
    pub admin: Address,
    pub balances: soroban_sdk::Map<Address, i128>,
    pub is_active: bool,
    pub configuration: ConfigurationData,
}

#[contracttype]
pub struct ConfigurationData {
    pub max_supply: i128,
    pub fee_rate: u32,
    pub owner: Address,
    pub metadata: Vec<Symbol>,
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, max_supply: i128) {
        admin.require_auth();
        let config = ConfigurationData {
            max_supply,
            fee_rate: 30, // 0.3%
            owner: admin.clone(),
            metadata: vec![&env, symbol_short!("VAULT")],
        };
        
        let storage = ComplexStorage {
            admin,
            balances: soroban_sdk::Map::new(&env),
            is_active: true,
            configuration: config,
        };
        
        env.storage().instance().set(&symbol_short!("STATE"), &storage);
        env.events().publish((symbol_short!("init"),), storage.is_active);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), soroban_sdk::Error> {
        from.require_auth();
        
        if amount <= 0 {
            panic!("Amount must be positive");
        }
        
        let mut state: ComplexStorage = env.storage().instance().get(&symbol_short!("STATE")).unwrap();
        
        if !state.is_active {
            panic!("Vault is not active");
        }
        
        let current_balance = state.balances.get(from.clone()).unwrap_or(0);
        let new_balance = current_balance + amount; // Potential arithmetic overflow
        
        state.balances.set(from.clone(), new_balance);
        env.storage().instance().set(&symbol_short!("STATE"), &state);
        
        env.events().publish((symbol_short!("dep"), from), amount);
        
        Ok(())
    }

    pub fn withdraw(env: Env, to: Address, amount: i128) {
        to.require_auth();
        
        let mut state: ComplexStorage = env.storage().instance().get(&symbol_short!("STATE")).unwrap();
        let current_balance = state.balances.get(to.clone()).expect("No balance found");
        
        if current_balance < amount {
            panic!("Insufficient balance");
        }
        
        // Risky arithmetic
        let new_balance = current_balance - amount;
        state.balances.set(to.clone(), new_balance);
        
        env.storage().instance().set(&symbol_short!("STATE"), &state);
        
        // Inconsistent event emission (different topic structure)
        env.events().publish((symbol_short!("with"), to.clone(), amount), new_balance);
    }
    
    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let state: ComplexStorage = env.storage().instance().get(&symbol_short!("STATE")).unwrap();
        state.admin.require_auth(); // Authorization gap if this was missing
        
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
    
    pub fn dangerous_unauth_transfer(env: Env, to: Address, amount: i128) {
        // Missing require_auth!
        let mut state: ComplexStorage = env.storage().instance().get(&symbol_short!("STATE")).unwrap();
        let admin_balance = state.balances.get(state.admin.clone()).unwrap_or(0);
        
        state.balances.set(state.admin.clone(), admin_balance - amount);
        state.balances.set(to.clone(), amount);
        
        env.storage().instance().set(&symbol_short!("STATE"), &state);
    }
}
"#;

fn bench_ast_parsing_and_rules(c: &mut Criterion) {
    let mut group = c.benchmark_group("Static Analysis Engine");
    
    // Benchmark the initialization of the analyzer
    group.bench_function("Analyzer Initialization", |b| {
        b.iter(|| {
            let config = SanctifyConfig::default();
            Analyzer::new(config)
        })
    });

    // Benchmark the full rule execution suite
    group.bench_function("Full AST Rule Execution", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);
        
        b.iter(|| {
            analyzer.run_rules(COMPLEX_CONTRACT_PAYLOAD)
        })
    });

    // Benchmark specific targeted rules
    group.bench_function("Auth Gaps Analysis", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);
        
        b.iter(|| {
            analyzer.scan_auth_gaps(COMPLEX_CONTRACT_PAYLOAD)
        })
    });

    group.bench_function("Panic & Unwrap Analysis", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);
        
        b.iter(|| {
            analyzer.scan_panics(COMPLEX_CONTRACT_PAYLOAD)
        })
    });

    group.bench_function("Ledger Size Analysis", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);
        
        b.iter(|| {
            analyzer.analyze_ledger_size(COMPLEX_CONTRACT_PAYLOAD)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_ast_parsing_and_rules);
criterion_main!(benches);
