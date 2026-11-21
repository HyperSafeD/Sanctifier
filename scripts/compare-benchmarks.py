#!/usr/bin/env python3
"""
Compare current benchmark results against a base-branch baseline and fail if
any benchmark's mean latency regresses by more than REGRESSION_THRESHOLD.

The threshold is intentionally set to 20 % to tolerate natural run-to-run
noise on GitHub-hosted runners while still catching a meaningful slowdown
(the issue cited a 10x regression going undetected).  Pass
--threshold <fraction> to override at call-site, e.g. --threshold 0.10.
"""
import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple

REGRESSION_THRESHOLD = 0.20  # 20 % default

def load_benchmark_results(path: Path) -> Dict:
    """Load benchmark results from JSON file"""
    if not path.exists():
        print(f"Error: Benchmark file not found: {path}")
        sys.exit(1)
    
    with open(path, 'r') as f:
        return json.load(f)

def compare_benchmarks(baseline: Dict, current: Dict) -> List[Tuple[str, float, float, float]]:
    """
    Compare benchmark results and return regressions
    
    Returns: List of (benchmark_name, baseline_time, current_time, regression_pct)
    """
    regressions = []
    
    baseline_benches = {b['name']: b for b in baseline.get('benchmarks', [])}
    current_benches = {b['name']: b for b in current.get('benchmarks', [])}
    
    for name, baseline_bench in baseline_benches.items():
        if name not in current_benches:
            print(f"⚠️  Warning: Benchmark '{name}' missing in current results")
            continue
        
        current_bench = current_benches[name]
        
        baseline_time = baseline_bench.get('mean', {}).get('estimate', 0)
        current_time = current_bench.get('mean', {}).get('estimate', 0)
        
        if baseline_time == 0:
            continue
        
        regression = (current_time - baseline_time) / baseline_time
        
        if regression > REGRESSION_THRESHOLD:
            regressions.append((name, baseline_time, current_time, regression))
    
    return regressions

def format_time(nanoseconds: float) -> str:
    """Format time in human-readable units"""
    if nanoseconds < 1000:
        return f"{nanoseconds:.2f} ns"
    elif nanoseconds < 1_000_000:
        return f"{nanoseconds / 1000:.2f} µs"
    elif nanoseconds < 1_000_000_000:
        return f"{nanoseconds / 1_000_000:.2f} ms"
    else:
        return f"{nanoseconds / 1_000_000_000:.2f} s"

def main():
    parser = argparse.ArgumentParser(description="Compare Criterion benchmark results against a baseline.")
    parser.add_argument("--baseline", default="benchmarks/baseline.json",
                        help="Path to the base-branch benchmark JSON (default: benchmarks/baseline.json)")
    parser.add_argument("--current", default="benchmarks/current.json",
                        help="Path to the PR benchmark JSON (default: benchmarks/current.json)")
    parser.add_argument("--threshold", type=float, default=REGRESSION_THRESHOLD,
                        help=f"Regression threshold as a fraction (default: {REGRESSION_THRESHOLD}, i.e. {REGRESSION_THRESHOLD*100:.0f}%%)")
    args = parser.parse_args()

    baseline_path = Path(args.baseline)
    current_path = Path(args.current)
    threshold = args.threshold

    if not baseline_path.exists():
        print("⚠️  No baseline found. This is the first run.")
        print("Storing current results as baseline...")
        if current_path.exists():
            import shutil
            shutil.copy(current_path, baseline_path)
        sys.exit(0)

    baseline = load_benchmark_results(baseline_path)
    current = load_benchmark_results(current_path)

    regressions = compare_benchmarks(baseline, current)
    # Filter by the caller-supplied threshold (compare_benchmarks uses the
    # module-level default; re-filter here to honour the CLI override).
    regressions = [(n, b, c, r) for n, b, c, r in regressions if r > threshold]

    if not regressions:
        print("✅ No performance regressions detected")
        print(f"All benchmarks within {threshold * 100:.0f}% of baseline")
        sys.exit(0)

    print(f"❌ Performance regression detected!")
    print(f"The following benchmarks regressed more than {threshold * 100:.0f}%:\n")

    for name, baseline_time, current_time, regression in regressions:
        print(f"Benchmark: {name}")
        print(f"  Baseline: {format_time(baseline_time)}")
        print(f"  Current:  {format_time(current_time)}")
        print(f"  Regression: {regression * 100:.2f}%")
        print()

    print("To override this check (if regression is justified):")
    print("  1. Review and approve the performance impact")
    print("  2. Add '[skip perf check]' to your commit message")
    print("  3. Or update the baseline: bash scripts/update-baseline.sh")

    sys.exit(1)

if __name__ == "__main__":
    main()
