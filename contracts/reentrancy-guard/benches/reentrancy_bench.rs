use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reentrancy_guard::ReentrancyGuard;
use soroban_sdk::{contract, testutils::Address as _, Address, Env};

#[contract]
pub struct BenchHarness;

fn bench_guard_overhead(c: &mut Criterion) {
    let env = Env::default();
    env.budget().reset_unlimited();
    let contract_id = env.register_contract(None, BenchHarness);
    let _caller = Address::generate(&env);

    c.bench_function("guard_enter_exit_per_invocation", |b| {
        b.iter(|| {
            env.budget().reset_tracker();
            env.as_contract(&contract_id, || {
                let guard = ReentrancyGuard::new(&env);
                guard.enter();
                black_box(());
                guard.exit();
            });
        });
    });
}

criterion_group!(benches, bench_guard_overhead);
criterion_main!(benches);
