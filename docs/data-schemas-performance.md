# Data + Schemas Performance Benchmarks & Budgets

This repository treats `data/` and `schemas/` as *production inputs*:
they ship in the repo, are loaded by tooling, and should remain fast to parse and validate.

This document describes:

- performance budgets enforced in CI
- a lightweight benchmark script for measuring JSON load/parse cost

## Budgets (enforced in CI)

Budgets are enforced by `tests/vulnerability_db/test_perf_budgets.py` and run in CI via:

```bash
python -m unittest discover -s tests/vulnerability_db -p "test_*.py"
```

Current budgets:

- Vulnerability DB JSON size: `<= 5 MiB` per file
- Vulnerability count: `<= 2000` entries per file
- Max regex pattern length: `<= 5000` chars per entry
- Total regex pattern length: `<= 500,000` chars per file
- Schema JSON size: `<= 512 KiB`

These budgets are intentionally conservative to avoid CI timeouts and keep local tooling responsive.

## Benchmarks (nightly + PR-visible)

The benchmark script measures JSON read/parse time and basic shape metrics:

```bash
python benchmarks/data_schemas_perf.py
```

To write a JSON report file:

```bash
python benchmarks/data_schemas_perf.py --output benchmarks/data_schemas_perf.json
```

The `Performance Benchmarks` workflow runs this script and records output for tracking.

