use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
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

        b.iter(|| analyzer.scan_auth_gaps(COMPLEX_CONTRACT_PAYLOAD))
    });

    // Benchmark specific targeted rules
    group.bench_function("Auth Gaps Analysis", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);

        b.iter(|| analyzer.scan_auth_gaps(COMPLEX_CONTRACT_PAYLOAD))
    });

    group.bench_function("Panic & Unwrap Analysis", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);

        b.iter(|| analyzer.scan_panics(COMPLEX_CONTRACT_PAYLOAD))
    });

    group.bench_function("Ledger Size Analysis", |b| {
        let config = SanctifyConfig::default();
        let analyzer = Analyzer::new(config);

        b.iter(|| analyzer.analyze_ledger_size(COMPLEX_CONTRACT_PAYLOAD))
    });

    group.finish();
}

// ── S004 ledger-size payloads ────────────────────────────────────────────────

/// Minimal struct — well under any ledger limit.
const SMALL_STRUCT: &str = r#"
    #[contracttype]
    pub struct Tiny { pub x: u32 }
"#;

/// Deep nesting — many `#[contracttype]` structs referencing each other.
const LARGE_NESTED_CONTRACT: &str = r#"
    #[contracttype] pub struct L1 { pub a: u64, pub b: u64, pub c: u64, pub d: u64 }
    #[contracttype] pub struct L2 { pub l1a: L1, pub l1b: L1, pub extra: u128 }
    #[contracttype] pub struct L3 { pub l2a: L2, pub l2b: L2, pub flag: bool }
    #[contracttype] pub struct L4 { pub l3: L3, pub count: u32, pub tag: u64 }
    #[contracttype] pub struct L5 { pub l4a: L4, pub l4b: L4, pub meta: u128 }
"#;

/// Many independent structs to stress throughput.
const MANY_STRUCTS: &str = r#"
    #[contracttype] pub struct A1  { pub v: u32 }
    #[contracttype] pub struct A2  { pub v: u32 }
    #[contracttype] pub struct A3  { pub v: u32 }
    #[contracttype] pub struct A4  { pub v: u32 }
    #[contracttype] pub struct A5  { pub v: u32 }
    #[contracttype] pub struct A6  { pub v: u32 }
    #[contracttype] pub struct A7  { pub v: u32 }
    #[contracttype] pub struct A8  { pub v: u32 }
    #[contracttype] pub struct A9  { pub v: u32 }
    #[contracttype] pub struct A10 { pub v: u32 }
    #[contracttype] pub struct A11 { pub v: u32 }
    #[contracttype] pub struct A12 { pub v: u32 }
    #[contracttype] pub struct A13 { pub v: u32 }
    #[contracttype] pub struct A14 { pub v: u32 }
    #[contracttype] pub struct A15 { pub v: u32 }
    #[contracttype] pub struct A16 { pub v: u32 }
    #[contracttype] pub struct A17 { pub v: u32 }
    #[contracttype] pub struct A18 { pub v: u32 }
    #[contracttype] pub struct A19 { pub v: u32 }
    #[contracttype] pub struct A20 { pub v: u32 }
"#;

// ── S004 ledger-size benchmarks ──────────────────────────────────────────────

/// Performance budget: ledger-size analysis of a single small struct must
/// complete under 5 ms (measured as a sanity-check floor, not a hard CI gate).
fn bench_ledger_size_s004(c: &mut Criterion) {
    let mut group = c.benchmark_group("S004 Ledger Size Rule");

    group.bench_function("small struct", |b| {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        b.iter(|| analyzer.analyze_ledger_size(SMALL_STRUCT))
    });

    group.bench_function("deep nested structs", |b| {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        b.iter(|| analyzer.analyze_ledger_size(LARGE_NESTED_CONTRACT))
    });

    group.bench_function("20 independent structs (throughput)", |b| {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        b.iter(|| analyzer.analyze_ledger_size(MANY_STRUCTS))
    });

    group.bench_function("complex vault contract", |b| {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        b.iter(|| analyzer.analyze_ledger_size(COMPLEX_CONTRACT_PAYLOAD))
    });

    // Strict-limit variant: measures overhead of threshold computation.
    group.bench_function("strict limit (50 bytes)", |b| {
        let config = SanctifyConfig {
            ledger_limit: 50,
            ..Default::default()
        };
        let analyzer = Analyzer::new(config);
        b.iter_batched(
            || LARGE_NESTED_CONTRACT,
            |src| analyzer.analyze_ledger_size(src),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── S008 event-analysis benchmark ────────────────────────────────────────────

fn bench_event_analysis_s008(c: &mut Criterion) {
    let mut group = c.benchmark_group("S008 Event Analysis");

    group.bench_function("complex vault contract", |b| {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        b.iter(|| analyzer.scan_events(COMPLEX_CONTRACT_PAYLOAD))
    });

    group.finish();
}

// ── S005 storage-collision benchmark ─────────────────────────────────────────

fn bench_storage_collision_s005(c: &mut Criterion) {
    let mut group = c.benchmark_group("S005 Storage Collision");

    group.bench_function("complex vault contract", |b| {
        let analyzer = Analyzer::new(SanctifyConfig::default());
        b.iter(|| analyzer.scan_storage_collisions(COMPLEX_CONTRACT_PAYLOAD))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ast_parsing_and_rules,
    bench_ledger_size_s004,
    bench_event_analysis_s008,
    bench_storage_collision_s005,
);
criterion_main!(benches);
