# Documentation Navigation Performance Benchmarks & Budgets

To ensure the Sanctifier documentation remains accessible and performant as the project scales, we enforce budgets on the main navigation hub (`DOCUMENTATION_INDEX.md`).

## Budgets (enforced in CI)

Budgets are checked by `tests/docs/test_nav_budgets.py` and run during the performance benchmark workflow.

Current budgets:

- **Index File Size**: `<= 20 KiB` (Current: ~11.5 KiB)
- **Total Navigation Links**: `<= 200` (Current: 63)
- **Max Nesting Depth**: `<= 5` levels (Current: 0)
- **Dead Internal Links**: `0` tolerance

## Benchmarking

You can manually trigger a navigation benchmark run with:

```bash
python3 benchmarks/docs_nav_perf.py
```

To generate a JSON report for CI or local comparison:

```bash
python3 benchmarks/docs_nav_perf.py --output benchmarks/docs_nav_perf.json
```

## Why these budgets?

1. **Accessibility**: Large, deeply nested indices are difficult to navigate for screen readers and mobile users.
2. **Maintenance**: A high number of links increases the risk of bit rot and dead links.
3. **Predictability**: Keeping the index lightweight ensures it loads instantly even in low-bandwidth environments.

## How to adopt

If you add a new document:
1. Link it in `DOCUMENTATION_INDEX.md` in the appropriate category.
2. Run `make lint` or specifically `python3 tests/docs/test_nav_budgets.py` to ensure you haven't exceeded any budgets or introduced dead links.
